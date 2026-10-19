//! Platform databases run synchronously on Rust workers; responses bypass the Android UI thread.
use super::assets::BundleAssets;
use ink_data::DataHost;
use ink_runtime::NativeHost;
use jni::Env;
use std::sync::{Arc, OnceLock};

#[cfg(feature = "store")]
mod system {
    use ink_data::{Cancellation, DatabaseBackend, DatabaseConnection, Failure};
    use ink_runtime::NativeCall;
    use jni::{
        Env, JavaVM, jni_sig, jni_str,
        objects::{JObject, JObjectArray, JString, JValue},
        refs::Global,
    };
    use std::time::Instant;

    pub(super) struct Backend {
        vm: JavaVM,
        factory: Global<JObject<'static>>,
    }

    impl Backend {
        pub(super) fn new(
            env: &mut Env<'_>,
            assets: &JObject<'_>,
            store_path: &str,
            files_path: &str,
        ) -> jni::errors::Result<Self> {
            let store = JString::from_str(env, store_path)?;
            let files = JString::from_str(env, files_path)?;
            let factory = env.new_object(
                jni_str!("com/vandam/ink/InkDatabases"),
                jni_sig!(
                    "(Landroid/content/res/AssetManager;Ljava/lang/String;Ljava/lang/String;)V"
                ),
                &[
                    JValue::Object(assets),
                    JValue::Object(&store),
                    JValue::Object(&files),
                ],
            )?;
            Ok(Self {
                vm: env.get_java_vm()?,
                factory: env.new_global_ref(factory)?,
            })
        }
    }

    impl DatabaseBackend for Backend {
        fn connect(&self, store: bool) -> Result<Box<dyn DatabaseConnection>, Failure> {
            let object = self
                .vm
                .attach_current_thread(|env| -> jni::errors::Result<_> {
                    let connection = env
                        .call_method(
                            &self.factory,
                            jni_str!("connect"),
                            jni_sig!("(Z)Lcom/vandam/ink/InkDatabaseConnection;"),
                            &[JValue::Bool(store)],
                        )?
                        .l()?;
                    env.new_global_ref(connection)
                })
                .map_err(Failure::unexpected)?;
            Ok(Box::new(Connection {
                vm: self.vm.clone(),
                object,
            }))
        }

        fn cancellation(&self) -> Result<Cancellation, Failure> {
            let signal = self
                .vm
                .attach_current_thread(|env| -> jni::errors::Result<_> {
                    let signal = env.new_object(
                        jni_str!("android/os/CancellationSignal"),
                        jni_sig!("()V"),
                        &[],
                    )?;
                    env.new_global_ref(signal)
                })
                .map_err(Failure::unexpected)?;
            let vm = self.vm.clone();
            Ok(Cancellation::with_interrupt(signal, move |signal| {
                let _ = vm.attach_current_thread(|env| {
                    env.call_method(signal, jni_str!("cancel"), jni_sig!("()V"), &[])
                        .map(|_| ())
                });
            }))
        }
    }

    struct Connection {
        vm: JavaVM,
        object: Global<JObject<'static>>,
    }

    impl DatabaseConnection for Connection {
        fn execute(
            &mut self,
            call: &NativeCall,
            cancellation: &Cancellation,
            _: Instant,
        ) -> Result<String, Failure> {
            let [value, kind, message] = self.vm.attach_current_thread(|env| -> jni::errors::Result<_> {
                let operation = JString::from_str(env, &call.operation)?;
                let payload = JString::from_str(env, &call.payload.to_string())?;
                let null = JObject::null();
                let signal = cancellation.context::<Global<JObject<'static>>>().map(|signal| signal.as_ref()).unwrap_or(&null);
                let result = env.call_method(&self.object, jni_str!("execute"),
                    jni_sig!("(Ljava/lang/String;Ljava/lang/String;Landroid/os/CancellationSignal;)[Ljava/lang/String;"),
                    &[JValue::Object(&operation), JValue::Object(&payload), JValue::Object(signal)])?.l()?;
                let result = env.cast_local::<JObjectArray<JString>>(result)?;
                let mut values = [None, None, None];
                for (index, slot) in values.iter_mut().enumerate() {
                    let value = result.get_element(env, index)?;
                    *slot = if value.is_null() { None } else { Some(value.try_to_string(env)?) };
                }
                Ok(values)
            }).map_err(Failure::unexpected)?;
            if let Some(value) = value {
                return Ok(value);
            }
            let kind = match kind.as_deref() {
                Some("protocol") => "protocol",
                _ => "unexpected",
            };
            Err(Failure(
                kind,
                message.unwrap_or_else(|| "Database operation failed".into()),
                false,
            ))
        }

        fn discard(&mut self, id: &str) {
            let _ = self
                .vm
                .attach_current_thread(|env| -> jni::errors::Result<_> {
                    let id = JString::from_str(env, id)?;
                    env.call_method(
                        &self.object,
                        jni_str!("discard"),
                        jni_sig!("(Ljava/lang/String;)V"),
                        &[JValue::Object(&id)],
                    )?;
                    Ok(())
                });
        }
    }

    impl Drop for Connection {
        fn drop(&mut self) {
            let _ = self.vm.attach_current_thread(|env| {
                env.call_method(&self.object, jni_str!("close"), jni_sig!("()V"), &[])
                    .map(|_| ())
            });
        }
    }
}

/// Foreground and background runtimes share one store worker and its change notifications.
pub(super) fn host(
    env: &mut Env<'_>,
    assets: &BundleAssets,
    store_path: String,
    files_path: String,
) -> jni::errors::Result<Arc<dyn NativeHost>> {
    static HOST: OnceLock<Arc<DataHost>> = OnceLock::new();
    if let Some(host) = HOST.get() {
        return Ok(host.clone());
    }
    #[cfg(feature = "store")]
    let host = DataHost::with_backend(Arc::new(system::Backend::new(
        env,
        assets.java_manager(),
        &store_path,
        &files_path,
    )?));
    #[cfg(not(feature = "store"))]
    let host = {
        let _ = (env, assets, store_path, files_path);
        DataHost::crypto_only()
    };
    Ok(HOST.get_or_init(|| host).clone())
}
