#![cfg(feature = "jni")]

use jni::objects::{JByteArray, JClass, JObject};
use jni::sys::{jboolean, jfloat, jint, jlong, JNI_TRUE};
use jni::JNIEnv;
use std::sync::Mutex;

use crate::{
    BehaviorGuard, KeystrokeEvent, MotionEvent, RawEvent, SwipeEvent, TouchEvent,
};

fn lock_guard(ptr: jlong) -> Option<std::sync::MutexGuard<'static, BehaviorGuard>> {
    if ptr == 0 {
        return None;
    }
    let mutex = unsafe { &*(ptr as *const Mutex<BehaviorGuard>) };
    mutex.lock().ok()
}

// ── Lifecycle ────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeCreate(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let guard = BehaviorGuard::new();
    let mutex = Box::new(Mutex::new(guard));
    Box::into_raw(mutex) as jlong
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle != 0 {
        unsafe { drop(Box::from_raw(handle as *mut Mutex<BehaviorGuard>)) };
    }
}

// ── Session ──────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeStartSession(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    lock_guard(handle)
        .and_then(|mut g| g.start_session().ok())
        .map(|_| JNI_TRUE)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeEndSession(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jint {
    lock_guard(handle)
        .and_then(|mut g| g.end_session().ok())
        .map(|outcome| match outcome {
            crate::SessionOutcome::Enrolling { sessions_remaining } => -(sessions_remaining as i32),
            crate::SessionOutcome::EnrollmentComplete => 0,
            crate::SessionOutcome::Scored(s) => (s.score * 1000.0) as i32,
        })
        .unwrap_or(-999)
}

// ── Event ingestion ──────────────────────────────────────────────────────────

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeAddKeystroke(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    down_ms: jlong,
    up_ms: jlong,
    flight_ms: jlong,
    is_correction: jboolean,
) {
    if let Some(mut g) = lock_guard(handle) {
        let _ = g.add_event(RawEvent::Keystroke(KeystrokeEvent {
            down_ms: down_ms as u64,
            up_ms: up_ms as u64,
            flight_ms: if flight_ms < 0 { None } else { Some(flight_ms as u64) },
            is_correction: is_correction != 0,
        }));
    }
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeAddTouch(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    down_ms: jlong,
    up_ms: jlong,
    x: jfloat,
    y: jfloat,
    pressure: jfloat,
    area: jfloat,
) {
    if let Some(mut g) = lock_guard(handle) {
        let _ = g.add_event(RawEvent::Touch(TouchEvent {
            down_ms: down_ms as u64,
            up_ms: up_ms as u64,
            x,
            y,
            pressure,
            area,
        }));
    }
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeAddSwipe(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    start_ms: jlong,
    end_ms: jlong,
    start_x: jfloat,
    start_y: jfloat,
    end_x: jfloat,
    end_y: jfloat,
    peak_velocity: jfloat,
) {
    if let Some(mut g) = lock_guard(handle) {
        let _ = g.add_event(RawEvent::Swipe(SwipeEvent {
            start_ms: start_ms as u64,
            end_ms: end_ms as u64,
            start_x,
            start_y,
            end_x,
            end_y,
            peak_velocity,
        }));
    }
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeAddMotion(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    timestamp_ms: jlong,
    gyro_x: jfloat,
    gyro_y: jfloat,
    gyro_z: jfloat,
    accel_x: jfloat,
    accel_y: jfloat,
    accel_z: jfloat,
) {
    if let Some(mut g) = lock_guard(handle) {
        let _ = g.add_event(RawEvent::Motion(MotionEvent {
            timestamp_ms: timestamp_ms as u64,
            gyro_x,
            gyro_y,
            gyro_z,
            accel_x,
            accel_y,
            accel_z,
        }));
    }
}

// ── Profile persistence ──────────────────────────────────────────────────────

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeExportProfile<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
    handle: jlong,
    key: JByteArray<'a>,
) -> JObject<'a> {
    let key_bytes: Vec<u8> = env.convert_byte_array(&key).unwrap_or_default();
    if key_bytes.len() != 32 {
        return JObject::null();
    }
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&key_bytes);
    let blob = lock_guard(handle)
        .and_then(|g| g.export_profile(&key_arr).ok().flatten());

    match blob {
        Some(b) => env.byte_array_from_slice(&b).map(JObject::from).unwrap_or(JObject::null()),
        None => JObject::null(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeImportProfile<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
    handle: jlong,
    blob: JByteArray<'a>,
    key: JByteArray<'a>,
) -> jboolean {
    let blob_bytes: Vec<u8> = env.convert_byte_array(&blob).unwrap_or_default();
    let key_bytes:  Vec<u8> = env.convert_byte_array(&key).unwrap_or_default();
    if key_bytes.len() != 32 {
        return 0;
    }
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&key_bytes);
    lock_guard(handle)
        .and_then(|mut g| g.import_profile(&blob_bytes, &key_arr).ok())
        .map(|_| JNI_TRUE)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeIsEnrolled(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    lock_guard(handle)
        .map(|g| if g.is_enrolled() { JNI_TRUE } else { 0 })
        .unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeIsModelReady(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jboolean {
    lock_guard(handle)
        .map(|g| if g.is_model_ready() { JNI_TRUE } else { 0 })
        .unwrap_or(0)
}

// ── Model persistence ────────────────────────────────────────────────────────

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeExportModel<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
    handle: jlong,
) -> JObject<'a> {
    let blob = lock_guard(handle).and_then(|g| g.export_model().ok().flatten());
    match blob {
        Some(b) => env.byte_array_from_slice(&b).map(JObject::from).unwrap_or(JObject::null()),
        None => JObject::null(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_behaviorgaurd_BehaviorGuard_nativeImportModel<'a>(
    env: JNIEnv<'a>,
    _class: JClass,
    handle: jlong,
    bytes: JByteArray<'a>,
) -> jboolean {
    let bytes_u8: Vec<u8> = env.convert_byte_array(&bytes).unwrap_or_default();
    lock_guard(handle)
        .and_then(|mut g| g.import_model(&bytes_u8).ok())
        .map(|_| JNI_TRUE)
        .unwrap_or(0)
}
