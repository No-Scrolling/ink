use jni::objects::{JByteArray, JByteBuffer, JClass};
use jni::sys::jint;
use jni::{EnvUnowned, jni_str};

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_vandam_ink_InkDirectPhotoCamera_nativeReviewPixels<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    y: JByteBuffer<'local>,
    y_row: jint,
    y_pixel: jint,
    u: JByteBuffer<'local>,
    u_row: jint,
    u_pixel: jint,
    v: JByteBuffer<'local>,
    v_row: jint,
    v_pixel: jint,
    source_width: jint,
    source_height: jint,
    width: jint,
    height: jint,
    rotation: jint,
) -> JByteArray<'local> {
    env.with_env(|env| {
        let valid = source_width > 0
            && source_height > 0
            && source_width % 2 == 0
            && source_height % 2 == 0
            && width > 0
            && height > 0
            && width <= 1440
            && height <= 1440
            && matches!(rotation, 0 | 90 | 180 | 270)
            && [y_row, y_pixel, u_row, u_pixel, v_row, v_pixel]
                .iter()
                .all(|&stride| stride > 0);
        if !valid {
            env.throw_new(
                jni_str!("java/lang/IllegalArgumentException"),
                jni_str!("Invalid camera dimensions"),
            )?;
            return Err(jni::errors::Error::JavaException);
        }
        let sw = source_width as usize;
        let sh = source_height as usize;
        let width = width as usize;
        let height = height as usize;
        let buffers = [y, u, v];
        let rows = [y_row as usize, u_row as usize, v_row as usize];
        let steps = [y_pixel as usize, u_pixel as usize, v_pixel as usize];
        let mut planes = [&[][..]; 3];
        for i in 0..3 {
            let capacity = env.get_direct_buffer_capacity(&buffers[i])?;
            let pointer = env.get_direct_buffer_address(&buffers[i])?;
            let (w, h) = if i == 0 { (sw, sh) } else { (sw / 2, sh / 2) };
            let required = (h - 1)
                .checked_mul(rows[i])
                .and_then(|offset| (w - 1).checked_mul(steps[i])?.checked_add(offset))
                .and_then(|offset| offset.checked_add(1));
            if required.is_none_or(|required| required > capacity) {
                env.throw_new(
                    jni_str!("java/lang/IllegalArgumentException"),
                    jni_str!("Invalid camera plane strides"),
                )?;
                return Err(jni::errors::Error::JavaException);
            }
            // The Image and its direct buffer slices remain alive for this synchronous call.
            // Stride bounds are checked above; none of these slices escape the call.
            planes[i] = unsafe { std::slice::from_raw_parts(pointer, capacity) };
        }
        let (upright_w, upright_h) = if rotation % 180 == 0 {
            (sw, sh)
        } else {
            (sh, sw)
        };
        let columns: Vec<_> = (0..width)
            .map(|x| (2 * x + 1) * upright_w / (2 * width))
            .collect();
        let mut rgba = vec![0u8; width * height * 4];
        for (row, output) in rgba.chunks_exact_mut(width * 4).enumerate() {
            let uy = (2 * row + 1) * upright_h / (2 * height);
            let (base_x, base_y, step_x, step_y) = match rotation {
                90 => (uy, sh - 1, 0isize, -1isize),
                180 => (sw - 1, sh - 1 - uy, -1, 0),
                270 => (sw - 1 - uy, 0, 0, 1),
                _ => (0, uy, 1, 0),
            };
            for (&ux, pixel) in columns.iter().zip(output.chunks_exact_mut(4)) {
                let sx = (base_x as isize + ux as isize * step_x) as usize;
                let sy = (base_y as isize + ux as isize * step_y) as usize;
                let y = i32::from(planes[0][sy * rows[0] + sx * steps[0]]) - 16;
                let u = i32::from(planes[1][sy / 2 * rows[1] + sx / 2 * steps[1]]) - 128;
                let v = i32::from(planes[2][sy / 2 * rows[2] + sx / 2 * steps[2]]) - 128;
                let luminance = 298 * y.max(0);
                pixel[0] = ((luminance + 409 * v + 128) >> 8).clamp(0, 255) as u8;
                pixel[1] = ((luminance - 100 * u - 208 * v + 128) >> 8).clamp(0, 255) as u8;
                pixel[2] = ((luminance + 516 * u + 128) >> 8).clamp(0, 255) as u8;
                pixel[3] = 255;
            }
        }
        env.byte_array_from_slice(&rgba)
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
