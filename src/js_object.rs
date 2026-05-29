/// Pointer type for JS allocated object.
/// Consider this as a Box, but pointing into JS memory.
/// -1 is null, -2 is undefined.
#[repr(transparent)]
pub struct JsObject(i32);

impl JsObject {
    /// Get a weak reference to JS memory.
    /// No guarantees against JS garbage collection.
    pub fn weak(&self) -> JsObjectWeak {
        JsObjectWeak(self.0)
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct JsObjectWeak(i32);

impl Drop for JsObject {
    fn drop(&mut self) {
        unsafe {
            js_free_object(self.weak());
        }
    }
}

#[cfg_attr(target_arch = "wasm32", link(wasm_import_module = "env"))]
unsafe extern "C" {
    fn js_create_string(buf: *const u8, max_len: u32) -> JsObject;
    fn js_create_buffer(buf: *const u8, max_len: u32) -> JsObject;
    fn js_create_object() -> JsObject;
    fn js_free_object(js_object: JsObjectWeak);
    fn js_unwrap_to_str(js_object: JsObjectWeak, buf: *mut u8, max_len: u32);
    fn js_unwrap_to_buf(js_object: JsObjectWeak, buf: *mut u8, max_len: u32);
    fn js_string_length(js_object: JsObjectWeak) -> u32;
    fn js_buf_length(js_object: JsObjectWeak) -> u32;
    fn js_have_field(js_object: JsObjectWeak, buf: *mut u8, len: u32) -> bool;
    fn js_field(js_object: JsObjectWeak, buf: *mut u8, len: u32) -> JsObject;
    fn js_field_f32(js_object: JsObjectWeak, buf: *mut u8, len: u32) -> f32;
    fn js_field_u32(js_object: JsObjectWeak, buf: *mut u8, len: u32) -> u32;
    fn js_set_field_string(
        js_object: JsObjectWeak,
        buf: *mut u8,
        len: u32,
        data_buf: *mut u8,
        data_len: u32,
    );
    fn js_set_field_f32(js_object: JsObjectWeak, buf: *mut u8, len: u32, data: f32);
    fn js_set_field_u32(js_object: JsObjectWeak, buf: *mut u8, len: u32, data: u32);
}

impl JsObject {
    pub fn string(string: &str) -> JsObject {
        unsafe { js_create_string(string.as_ptr(), string.len() as _) }
    }

    pub fn buffer(data: &[u8]) -> JsObject {
        unsafe { js_create_buffer(data.as_ptr(), data.len() as _) }
    }

    pub fn object() -> JsObject {
        unsafe { js_create_object() }
    }

    pub fn to_string(&self, buf: &mut String) {
        let len = unsafe { js_string_length(self.weak()) };

        if len as usize > buf.len() {
            buf.reserve(len as usize - buf.len());
        }
        unsafe { buf.as_mut_vec().set_len(len as usize) };
        unsafe { js_unwrap_to_str(self.weak(), buf.as_mut_vec().as_mut_ptr(), len) };
    }

    pub fn to_byte_buffer(&self, buf: &mut Vec<u8>) {
        let len = unsafe { js_buf_length(self.weak()) };
        buf.resize(len as usize, 0u8);
        unsafe { js_unwrap_to_buf(self.weak(), buf.as_mut_ptr(), len) };
    }

    pub fn field(&self, field: &str) -> JsObject {
        unsafe { js_field(self.weak(), field.as_ptr() as _, field.len() as _) }
    }

    pub fn field_u32(&self, field: &str) -> u32 {
        unsafe { js_field_u32(self.weak(), field.as_ptr() as _, field.len() as _) }
    }

    pub fn have_field(&self, field: &str) -> bool {
        unsafe { js_have_field(self.weak(), field.as_ptr() as _, field.len() as _) }
    }

    pub fn field_f32(&self, field: &str) -> f32 {
        unsafe { js_field_f32(self.weak(), field.as_ptr() as _, field.len() as _) }
    }

    pub fn set_field_f32(&self, field: &str, data: f32) {
        unsafe { js_set_field_f32(self.weak(), field.as_ptr() as _, field.len() as _, data) }
    }

    pub fn set_field_u32(&self, field: &str, data: u32) {
        unsafe { js_set_field_u32(self.weak(), field.as_ptr() as _, field.len() as _, data) }
    }

    pub fn set_field_string(&self, field: &str, data: &str) {
        unsafe {
            js_set_field_string(
                self.weak(),
                field.as_ptr() as _,
                field.len() as _,
                data.as_ptr() as _,
                data.len() as _,
            )
        }
    }

    pub fn is_nil(&self) -> bool {
        self.0 == Self::NULL.0
    }

    pub fn is_undefined(&self) -> bool {
        self.0 == Self::UNDEFINED.0
    }

    pub const NULL: Self = Self(-1);
    pub const UNDEFINED: Self = Self(-2);
}
