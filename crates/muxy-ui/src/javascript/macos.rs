//! The system's `JavaScriptCore`.

#![allow(
    deprecated,
    reason = "Use the C API without Objective-C JSContext wrappers"
)]

pub(super) use objc2_javascript_core::{
    JSCheckScriptSyntax, JSContextGetGlobalObject, JSContextRef, JSEvaluateScript,
    JSGlobalContextCreate, JSGlobalContextRef, JSGlobalContextRelease,
    JSObjectMakeFunctionWithCallback, JSObjectRef, JSObjectSetProperty,
    JSStringCreateWithCharacters, JSStringGetCharactersPtr, JSStringGetLength, JSStringRef,
    JSStringRelease, JSValueIsNumber, JSValueMakeString, JSValueRef, JSValueToNumber,
    JSValueToStringCopy, kJSPropertyAttributeDontDelete, kJSPropertyAttributeReadOnly,
};

/// The framework is part of the system, so there is nothing to load.
#[allow(
    clippy::unnecessary_wraps,
    reason = "The Linux binding, which shares this signature, can fail"
)]
pub(super) fn load() -> Result<(), String> {
    Ok(())
}
