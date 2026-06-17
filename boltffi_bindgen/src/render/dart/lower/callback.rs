use boltffi_ffi_rules::{callable::ExecutionKind, transport::ValueReturnStrategy};

use crate::{
    ir::{
        AbiCallbackInvocation, AbiCallbackMethod, AbiParam, CallbackId, CallbackKind,
        CallbackMethodDef, CallbackTraitDef, ParamDef, ParamRole, ReadSeq, Transport,
    },
    render::dart::{
        DartCallback, DartCallbackMethod, DartFFIClosureParam, DartFFIFunctionParamSig,
        DartFFIFunctionSig, DartFFIIntType, DartFFIType, DartFunctionSig, DartType,
        NamingConvention,
    },
};

impl<'a> super::DartLowerer<'a> {
    fn abi_callback_for(&self, id: &CallbackId) -> Option<&AbiCallbackInvocation> {
        self.abi.callbacks.iter().find(|cb| cb.callback_id == *id)
    }

    fn lower_closure_param(
        &self,
        param_def: &ParamDef,
        transport: &Transport,
        decode_ops: &Option<ReadSeq>,
    ) -> DartFFIClosureParam {
        let passing = self.param_passing_from_transport(transport);
        let ty = DartType::from_type_expr(&param_def.type_expr, &self.ffi.catalog);

        DartFFIClosureParam {
            name: param_def.name.to_string(),
            recv: passing,
            read_seq: decode_ops.clone(),
            ty,
        }
    }

    pub(super) fn lower_closure_params(
        &self,
        param_defs: &[ParamDef],
        abi_params: &[AbiParam],
    ) -> Vec<DartFFIClosureParam> {
        std::iter::zip(param_defs, abi_params)
            .map(|(param_def, abi_param)| {
                let ParamRole::Input {
                    transport,
                    decode_ops,
                    ..
                } = &abi_param.role
                else {
                    unreachable!();
                };

                self.lower_closure_param(param_def, transport, decode_ops)
            })
            .collect()
    }

    fn lower_callback_method(
        &self,
        cb: &CallbackMethodDef,
        abi_meth: &AbiCallbackMethod,
    ) -> DartCallbackMethod {
        let params = self.lower_closure_params(&cb.params, &abi_meth.params);

        let mut ffi_args = vec![DartFFIFunctionParamSig {
            name: "_p$handle".to_string(),
            ty: DartFFIType::Int(DartFFIIntType::Uint64),
        }];

        ffi_args.extend(
            abi_meth.params[1..]
                .iter()
                .map(DartFFIFunctionParamSig::from_abi_param),
        );

        match abi_meth.execution_kind {
            ExecutionKind::Sync => {
                ffi_args.push(DartFFIFunctionParamSig {
                    name: "_p$outStatus".to_string(),
                    ty: DartFFIType::Pointer(Box::new(DartFFIType::Status)),
                });
            }
            ExecutionKind::Async => {
                let mut callback_params = vec![
                    // async context handle
                    DartFFIFunctionParamSig {
                        name: String::new(),
                        ty: DartFFIType::Int(DartFFIIntType::Uint64),
                    },
                ];

                if !matches!(
                    abi_meth.returns.return_contract().value_strategy(),
                    ValueReturnStrategy::Void
                ) {
                    callback_params.extend([
                        // result bytes ptr
                        DartFFIFunctionParamSig {
                            name: String::new(),
                            ty: DartFFIType::Pointer(Box::new(DartFFIType::Int(
                                DartFFIIntType::Uint8,
                            ))),
                        },
                        // result bytes len
                        DartFFIFunctionParamSig {
                            name: String::new(),
                            ty: DartFFIType::Int(DartFFIIntType::UintPtr),
                        },
                    ]);
                }
                callback_params.push(
                    // This should be FFIStatus but we choose i32 as it's a valid repr
                    DartFFIFunctionParamSig {
                        name: String::new(),
                        ty: DartFFIType::Int(DartFFIIntType::Int32),
                    },
                );

                ffi_args.extend([
                    DartFFIFunctionParamSig {
                        name: "_p$callback".to_string(),
                        ty: DartFFIType::NativeFunction {
                            sig: Box::new(DartFFIFunctionSig {
                                args: callback_params,
                                ret: DartFFIType::Void,
                            }),
                        },
                    },
                    DartFFIFunctionParamSig {
                        name: "_p$asyncCtx".to_string(),
                        ty: DartFFIType::Int(DartFFIIntType::Uint64),
                    },
                ]);
            }
        };

        DartCallbackMethod {
            name: NamingConvention::function_name(cb.id.as_str()),
            sig: DartFunctionSig::from_params_return_def(
                &cb.params,
                &cb.returns,
                &self.ffi.catalog,
            ),
            ffi_sig: DartFFIFunctionSig {
                args: ffi_args,
                ret: DartFFIType::from_return_shape_and_error_transport(
                    &abi_meth.returns,
                    &abi_meth.error,
                ),
            },
            params,
            // ret_ty: DartType::from_return_def(&cb.returns, &self.ffi.catalog),
            kind: cb.execution_kind,
        }
    }

    fn lower_one_callback(&self, cb_def: &CallbackTraitDef) -> DartCallback {
        let abi_cb = self.abi_callback_for(&cb_def.id).unwrap();

        let class_name = NamingConvention::class_name(cb_def.id.as_str());
        let impl_class_name = format!("_I${}", class_name);
        let vtable_struct_name = format!(
            "_I${}",
            NamingConvention::class_name(abi_cb.vtable_type.as_str())
        );

        let methods = std::iter::zip(&cb_def.methods, &abi_cb.methods)
            .map(|(meth_def, abi_meth)| self.lower_callback_method(meth_def, abi_meth))
            .collect();

        DartCallback {
            class_name,
            impl_class_name,
            vtable_struct_name,
            methods,
        }
    }

    pub(super) fn lower_callbacks(&self) -> Vec<DartCallback> {
        self.ffi
            .catalog
            .all_callbacks()
            .filter(|cb| matches!(cb.kind, CallbackKind::Trait))
            .map(|cb| self.lower_one_callback(cb))
            .collect()
    }
}
