//! Activity discovery via `ActivityThread` internals.

use jni::{Env, JValue, jni_sig, jni_str, objects::JObject};

use crate::log_info;

/// `ActivityThread.currentActivityThread()`
pub fn get_activity_thread<'local>(env: &mut Env<'local>) -> jni::errors::Result<JObject<'local>> {
    let cls = env.find_class(jni_str!("android/app/ActivityThread"))?;

    let thread = env
        .call_static_method(
            cls,
            jni_str!("currentActivityThread"),
            jni_sig!("()Landroid/app/ActivityThread;"),
            &[],
        )?
        .l()?;

    Ok(thread)
}

/// Walk `mActivities` (ArrayMap of ActivityClientRecord) and return the first
/// alive, non-finishing Activity with a valid window token.
///
/// ```text
/// ActivityThread
///   └── mActivities: ArrayMap
///         └── ActivityClientRecord.activity
/// ```
pub fn get_current_activity<'local>(env: &mut Env<'local>) -> jni::errors::Result<JObject<'local>> {
    let thread = get_activity_thread(env)?;

    let activities = env
        .get_field(
            &thread,
            jni_str!("mActivities"),
            jni_sig!("Landroid/util/ArrayMap;"),
        )?
        .l()?;

    if activities.is_null() {
        return Err(jni::errors::Error::NullPtr(
            "ActivityThread.mActivities is null",
        ));
    }

    let size = env
        .call_method(&activities, jni_str!("size"), jni_sig!("()I"), &[])?
        .i()?;

    for i in 0..size {
        let record = env
            .call_method(
                &activities,
                jni_str!("valueAt"),
                jni_sig!("(I)Ljava/lang/Object;"),
                &[JValue::Int(i)],
            )?
            .l()?;

        if record.is_null() {
            continue;
        }

        let activity = env
            .get_field(
                &record,
                jni_str!("activity"),
                jni_sig!("Landroid/app/Activity;"),
            )?
            .l()?;

        if activity.is_null()
            || env
                .call_method(&activity, jni_str!("isFinishing"), jni_sig!("()Z"), &[])?
                .z()?
            || env
                .call_method(&activity, jni_str!("isDestroyed"), jni_sig!("()Z"), &[])?
                .z()?
        {
            continue;
        }

        if get_window_token(env, &activity)?.is_null() {
            continue;
        }

        log_info!("[rust] Activity found");
        return Ok(activity);
    }

    Err(jni::errors::Error::NullPtr("no valid Activity found"))
}

/// `activity.getSystemService("window")`
pub fn get_window_manager<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let service = env.new_string("window")?;

    let wm = env
        .call_method(
            activity,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::Object(&service.into())],
        )?
        .l()?;

    Ok(wm)
}

/// `activity.getWindow().getDecorView().getWindowToken()`
pub fn get_window_token<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let window = env
        .call_method(
            activity,
            jni_str!("getWindow"),
            jni_sig!("()Landroid/view/Window;"),
            &[],
        )?
        .l()?;

    let decor = env
        .call_method(
            &window,
            jni_str!("getDecorView"),
            jni_sig!("()Landroid/view/View;"),
            &[],
        )?
        .l()?;

    let token = env
        .call_method(
            &decor,
            jni_str!("getWindowToken"),
            jni_sig!("()Landroid/os/IBinder;"),
            &[],
        )?
        .l()?;

    Ok(token)
}
