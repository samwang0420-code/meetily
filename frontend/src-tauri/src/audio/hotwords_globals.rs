// 离线会记 v0.5.0: 全局热词配置 (读多写少)
// 让 audio 路径 (retranscription + parallel processor) 用同一份当前设置

use std::sync::atomic::{AtomicPtr, Ordering};

static PACK: AtomicPtr<String> = AtomicPtr::new(std::ptr::null_mut());
static CUSTOM: AtomicPtr<String> = AtomicPtr::new(std::ptr::null_mut());

const PRODUCT_TERMS: &str = "离线会记,Meetily,SenseVoice,sense voice,FunASR,Paraformer,sherpa-onnx,ASR,Ollama,BlockNote,Tauri";

pub fn set(pack: String, custom: String) {
    let p_pack = Box::into_raw(Box::new(pack));
    let p_custom = Box::into_raw(Box::new(custom));
    let old_pack = PACK.swap(p_pack, Ordering::SeqCst);
    let old_custom = CUSTOM.swap(p_custom, Ordering::SeqCst);
    if !old_pack.is_null() {
        unsafe { drop(Box::from_raw(old_pack)); }
    }
    if !old_custom.is_null() {
        unsafe { drop(Box::from_raw(old_custom)); }
    }
}

/// Returns 'static str by leaking the String. We can't return &str from heap;
/// instead, we leak each String into 'static. To avoid leaks, this is meant
/// to be called once per process with stored state.
fn read(ptr: *const String) -> &'static str {
    if ptr.is_null() { return ""; }
    unsafe { (*ptr).as_str() }
}

pub fn current_pack() -> &'static str {
    read(PACK.load(Ordering::SeqCst))
}

pub fn current_custom() -> &'static str {
    read(CUSTOM.load(Ordering::SeqCst))
}

pub fn current_custom_with_product_terms() -> String {
    let pack = current_pack();
    let custom = current_custom();
    // 'none' means user has not picked an industry pack; treat as no domain.
    let domain = if pack == "none" || pack.is_empty() { None } else { Some(pack) };
    let merged = format!("{PRODUCT_TERMS},{custom}");
    crate::audio::industry_vocab::build_runtime_terms_for_domain(domain, &merged)
}
