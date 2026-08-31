package com.vandam.ink

internal fun createNotificationsAdapter(
    activity: MainActivity,
    update: (Long, String) -> Unit,
): NotificationsAdapter = NoNotificationsAdapter

private object NoNotificationsAdapter : NotificationsAdapter {
    override fun start() = Unit

    override fun stop() = Unit

    override fun execute(
        requestId: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) {
        complete(NativeResult.Failure(NativeErrorKind.UNAVAILABLE, "Notifications are not packaged", false))
    }

    override fun executeController(
        controller: Long,
        operation: String,
        payload: String,
        complete: NativeResultHandler,
    ) = execute(0, operation, payload, complete)

    override fun cancel(requestId: Long) = Unit

    override fun refreshEvents() = Unit

    override fun handleIntent(intent: android.content.Intent) = Unit
}
