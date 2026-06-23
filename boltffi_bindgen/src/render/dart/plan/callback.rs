use boltffi_ffi_rules::callable::ExecutionKind;

#[derive(Debug, Clone)]
pub struct DartNativeCallbackMethod {
    pub vtable_field_name: String,
    pub sig: super::DartFFIFunctionSig,
    pub params: Vec<super::DartFFIClosureParam>,
    pub return_type: super::DartFFIType,
    pub kind: ExecutionKind,
}

impl DartNativeCallbackMethod {
    pub fn is_async(&self) -> bool {
        matches!(self.kind, ExecutionKind::Async)
    }
}

#[derive(Debug, Clone)]
pub struct DartNativeCallback {
    pub vtable_struct_name: String,
    pub methods: Vec<DartNativeCallbackMethod>,
}

#[derive(Debug, Clone)]
pub struct DartCallbackMethod {
    pub name: String,
    pub sig: super::DartFunctionSig,
    pub ffi_sig: super::DartFFIFunctionSig,
    pub params: Vec<super::DartFFIClosureParam>,
    pub kind: ExecutionKind,
    pub returns: super::DartFFIClosureReturns,
}

impl DartCallbackMethod {
    pub fn is_async(&self) -> bool {
        matches!(self.kind, ExecutionKind::Async)
    }
}

#[derive(Debug, Clone)]
pub struct DartCallback {
    pub class_name: String,
    pub impl_class_name: String,
    pub vtable_struct_name: String,
    pub methods: Vec<DartCallbackMethod>,
}
