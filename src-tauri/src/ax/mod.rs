//! Minimal, polling-based wrapper around macOS AXUIElement for reading the
//! focused text field's value.
//!
//! We deliberately avoid `AXObserver` / `CFRunLoopSource` — those would need a
//! dedicated thread with a CFRunLoop, and the UX benefit over a 500ms poll is
//! imperceptible for a capture that runs at most once per dictation. The
//! polling loop lives in `crate::capture`.

#[cfg(target_os = "macos")]
mod imp {
    use core_foundation::base::{CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;
    use std::ptr;

    type AXUIElementRef = *const c_void;
    type AXError = i32;

    const K_AX_ERROR_SUCCESS: AXError = 0;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        fn CFRelease(cf: CFTypeRef);
        fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
        fn CFGetTypeID(cf: CFTypeRef) -> usize;
        fn CFStringGetTypeID() -> usize;
    }

    /// A retained AXUIElement. Releases on drop.
    pub struct AxElement {
        inner: AXUIElementRef,
    }

    // AXUIElement is thread-safe for retain/release/copy per Apple's docs, so
    // we can ship it across the polling task thread boundary.
    unsafe impl Send for AxElement {}
    unsafe impl Sync for AxElement {}

    impl AxElement {
        /// Returns the currently-focused UI element system-wide, if any.
        pub fn focused() -> Option<Self> {
            unsafe {
                let system_wide = AXUIElementCreateSystemWide();
                if system_wide.is_null() {
                    return None;
                }
                let attr = CFString::from_static_string("AXFocusedUIElement");
                let mut value: CFTypeRef = ptr::null();
                let err = AXUIElementCopyAttributeValue(
                    system_wide,
                    attr.as_concrete_TypeRef(),
                    &mut value,
                );
                CFRelease(system_wide);
                if err != K_AX_ERROR_SUCCESS || value.is_null() {
                    return None;
                }
                // Copy gives us a retained ref; we take ownership.
                Some(AxElement {
                    inner: value as AXUIElementRef,
                })
            }
        }

        /// Reads the element's `AXValue` attribute as a string, if present.
        pub fn read_value(&self) -> Option<String> {
            self.read_string_attr("AXValue")
        }

        /// Returns true if the element is a secure (password) text field.
        pub fn is_password_field(&self) -> bool {
            matches!(
                self.read_string_attr("AXSubrole").as_deref(),
                Some("AXSecureTextField")
            )
        }

        /// Returns true if two elements refer to the same underlying UI object.
        /// AX doesn't expose an equality API, so we compare pointer values —
        /// this is sufficient because `CFRetain` on the same element returns a
        /// ref that the kernel treats as identical for our polling purposes.
        pub fn is_same(&self, other: &AxElement) -> bool {
            self.inner == other.inner
        }

        fn read_string_attr(&self, attribute: &'static str) -> Option<String> {
            unsafe {
                let attr = CFString::from_static_string(attribute);
                let mut value: CFTypeRef = ptr::null();
                let err = AXUIElementCopyAttributeValue(
                    self.inner,
                    attr.as_concrete_TypeRef(),
                    &mut value,
                );
                if err != K_AX_ERROR_SUCCESS || value.is_null() {
                    return None;
                }
                // Confirm we got a CFString back; if not (e.g. AXValue on a
                // slider is a number), release and bail.
                if CFGetTypeID(value) != CFStringGetTypeID() {
                    CFRelease(value);
                    return None;
                }
                let cf_str: CFString =
                    CFString::wrap_under_create_rule(value as CFStringRef);
                Some(cf_str.to_string())
            }
        }
    }

    impl Clone for AxElement {
        fn clone(&self) -> Self {
            unsafe {
                let retained = CFRetain(self.inner);
                AxElement {
                    inner: retained as AXUIElementRef,
                }
            }
        }
    }

    impl Drop for AxElement {
        fn drop(&mut self) {
            unsafe {
                if !self.inner.is_null() {
                    CFRelease(self.inner);
                }
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    /// Cross-platform stub. Auto-capture is macOS-only for V1.
    pub struct AxElement;

    impl AxElement {
        pub fn focused() -> Option<Self> {
            None
        }
        pub fn read_value(&self) -> Option<String> {
            None
        }
        pub fn is_password_field(&self) -> bool {
            false
        }
        pub fn is_same(&self, _other: &AxElement) -> bool {
            false
        }
    }

    impl Clone for AxElement {
        fn clone(&self) -> Self {
            AxElement
        }
    }
}

pub use imp::AxElement;
