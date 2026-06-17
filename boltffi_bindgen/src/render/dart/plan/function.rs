use crate::{
    ir::{ClassId, EnumId, PrimitiveType, ReadSeq, RecordId, WriteSeq},
    render::dart::emit,
};

#[derive(Debug, Clone)]
pub enum DartFFIParamValue {
    Primitive(super::DartFFIPrimitiveType),
    Record(String),
    Enum,
}

impl DartFFIParamValue {
    pub fn from_primitive(primitive: PrimitiveType) -> Self {
        Self::Primitive(super::DartFFIPrimitiveType::from_primitive(primitive))
    }
}

#[derive(Debug, Clone)]
pub enum DartFFIParamBytes {
    Array(DartFFIParamValue),
    Record(String),
    UTF8,
}

impl DartFFIParamBytes {}

#[derive(Debug, Clone)]
pub struct DartFFIClosureParam {
    pub name: String,
    pub recv: DartFFIParamPassing,
    pub read_seq: Option<ReadSeq>,
    pub ty: super::DartType,
}

impl DartFFIClosureParam {
    pub fn buf_reader_name(&self) -> String {
        assert!(self.read_seq.is_some(), "ffi buffer parts");

        format!("l${}Buf", self.name)
    }

    pub fn ffi_param_ptr_name(&self) -> String {
        assert!(self.read_seq.is_some(), "ffi buffer parts");

        format!("{}Ptr", self.name)
    }

    pub fn ffi_param_len_name(&self) -> String {
        assert!(self.read_seq.is_some(), "ffi buffer parts");

        format!("{}Len", self.name)
    }

    pub fn wire_read_expr(&self) -> String {
        emit::emit_reader_read(
            self.read_seq.as_ref().expect("ffi buffer parts"),
            &self.buf_reader_name(),
        )
    }

    pub fn value_read_expr(&self) -> String {
        let DartFFIParamPassing::Value(value) = &self.recv else {
            panic!("value passsing")
        };

        match value {
            DartFFIParamValue::Primitive(..) => match &self.ty {
                super::DartType::Bool | super::DartType::Int | super::DartType::Double => {
                    self.name.clone()
                }
                super::DartType::Record(class) => {
                    format!("{}._m$fromStruct({})", class, self.name)
                }
                super::DartType::Enum(class) => {
                    format!("{}._m$fromValue({})", class, self.name)
                }
                super::DartType::Custom(_) => todo!(),
                _ => unreachable!(),
            },
            DartFFIParamValue::Record(class) => {
                format!("{}._m$fromStruct({})", class, self.name)
            }
            DartFFIParamValue::Enum => {
                let enum_class = match &self.ty {
                    super::DartType::Enum(class) => class,
                    _ => unreachable!(),
                };
                format!("{}._m$fromValue({})", enum_class, self.name)
            }
        }
    }

    pub fn bytes_read_expr(&self) -> String {
        let DartFFIParamPassing::Bytes(bytes) = &self.recv else {
            panic!("bytes passsing")
        };

        match bytes {
            DartFFIParamBytes::Array(value) => match value {
                DartFFIParamValue::Primitive(primitive) => match primitive {
                    super::DartFFIPrimitiveType::Bool => {
                        format!(
                            "_$$BoltFFIBoolList._m$fromUint8List({}.asTypedList({}).sublist(0).buffer.asUint8List())",
                            self.ffi_param_ptr_name(),
                            self.ffi_param_len_name()
                        )
                    }
                    super::DartFFIPrimitiveType::Int(int) => match int {
                        super::DartFFIIntType::Uint8 => {
                            format!(
                                "{}.asTypedList({}).sublist(0).buffer.asUint8List()",
                                self.ffi_param_ptr_name(),
                                self.ffi_param_len_name()
                            )
                        }
                        super::DartFFIIntType::Int8 => {
                            format!(
                                "{}.asTypedList({}).sublist(0).buffer.asInt8List()",
                                self.ffi_param_ptr_name(),
                                self.ffi_param_len_name()
                            )
                        }
                        super::DartFFIIntType::Uint16 => {
                            format!(
                                "{}.asTypedList({}).sublist(0).buffer.asUint16List()",
                                self.ffi_param_ptr_name(),
                                self.ffi_param_len_name()
                            )
                        }
                        super::DartFFIIntType::Int16 => {
                            format!(
                                "{}.asTypedList({}).sublist(0).buffer.asInt16List()",
                                self.ffi_param_ptr_name(),
                                self.ffi_param_len_name()
                            )
                        }
                        super::DartFFIIntType::Uint32 => {
                            format!(
                                "{}.asTypedList({}).sublist(0).buffer.asUint32List()",
                                self.ffi_param_ptr_name(),
                                self.ffi_param_len_name()
                            )
                        }
                        super::DartFFIIntType::Int32 => {
                            format!(
                                "{}.asTypedList({}).sublist(0).buffer.asInt32List()",
                                self.ffi_param_ptr_name(),
                                self.ffi_param_len_name()
                            )
                        }
                        super::DartFFIIntType::Uint64 | super::DartFFIIntType::UintPtr => {
                            format!(
                                "{}.asTypedList({}).sublist(0).buffer.asUint64List()",
                                self.ffi_param_ptr_name(),
                                self.ffi_param_len_name()
                            )
                        }
                        super::DartFFIIntType::Int64 | super::DartFFIIntType::IntPtr => {
                            format!(
                                "{}.asTypedList({}).sublist(0).buffer.asInt64List()",
                                self.ffi_param_ptr_name(),
                                self.ffi_param_len_name()
                            )
                        }
                    },
                    super::DartFFIPrimitiveType::Float(float) => match float {
                        super::DartFFIFloatType::Float32 => format!(
                            "{}.asTypedList({}).sublist(0).buffer.asFloat32List()",
                            self.ffi_param_ptr_name(),
                            self.ffi_param_len_name()
                        ),
                        super::DartFFIFloatType::Float64 => format!(
                            "{}.asTypedList({}).sublist(0).buffer.asFloat64List()",
                            self.ffi_param_ptr_name(),
                            self.ffi_param_len_name()
                        ),
                    },
                },
                DartFFIParamValue::Record(record) => format!(
                    "{}._m$fromStructPtr({}, {})",
                    record,
                    self.ffi_param_ptr_name(),
                    self.ffi_param_len_name(),
                ),
                DartFFIParamValue::Enum => String::new(),
            },
            DartFFIParamBytes::Record(record) => format!(
                "{}._m$fromStructPtr({}, {})",
                record,
                self.ffi_param_ptr_name(),
                self.ffi_param_len_name(),
            ),
            DartFFIParamBytes::UTF8 => format!(
                "$$convert.utf8.decode({}.asTypedList({}))",
                self.ffi_param_ptr_name(),
                self.ffi_param_len_name()
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DartFFIClosureDef {
    pub sig: super::DartFFIFunctionSig,
    pub params: Vec<DartFFIClosureParam>,
}

impl DartFFIClosureDef {
    // pub fn get_dart_params(&self) -> impl Iterator<Item = String> {
    //     self.params.iter().map(|p| match &p.recv {
    //         DartFFIParamPassing::Value(value) => ,
    //         DartFFIParamPassing::WireEncoded => todo!(),
    //         DartFFIParamPassing::Bytes(bytes) => todo!(),
    //         DartFFIParamPassing::Closure(..) => todo!(),
    //         DartFFIParamPassing::ClassHandle => todo!(),
    //         DartFFIParamPassing::CallbackHandle(_) => todo!(),
    //     })
    // }
}

#[derive(Debug, Clone)]
pub enum DartFFIParamPassing {
    /// ints, floats, bools, records, enums, ...
    Value(DartFFIParamValue),
    /// enums, records, strings, lists
    WireEncoded,
    /// arrays of (ints, floats, bools, records, ...), strings, records
    Bytes(DartFFIParamBytes),
    /// closures
    Closure(DartFFIClosureDef),
    /// class handle
    ClassHandle,
    /// callback handle
    CallbackHandle { class: String, nullable: bool },
}

impl DartFFIParamPassing {
    pub fn primitive_value(primitive: PrimitiveType) -> Self {
        DartFFIParamPassing::Value(DartFFIParamValue::from_primitive(primitive))
    }

    pub fn record_value(record: String) -> Self {
        DartFFIParamPassing::Value(DartFFIParamValue::Record(record))
    }

    pub fn primitive_bytes(primitive: PrimitiveType) -> Self {
        DartFFIParamPassing::Bytes(DartFFIParamBytes::Array(DartFFIParamValue::from_primitive(
            primitive,
        )))
    }

    pub fn record_bytes(record: String) -> Self {
        DartFFIParamPassing::Bytes(DartFFIParamBytes::Array(DartFFIParamValue::Record(record)))
    }

    pub fn utf8_bytes() -> Self {
        DartFFIParamPassing::Bytes(DartFFIParamBytes::UTF8)
    }
}

#[derive(Debug, Clone)]
pub enum DartFFIReturnsRecv {
    /// ints, floats, bools, records, closures, ...
    Value,
}

#[derive(Debug, Clone)]
pub struct DartFFIFunctionReturns {
    pub ty: super::DartType,
    pub passing: DartFFIReturnsRecv,
}

#[derive(Debug, Clone)]
pub struct DartFunctionParam {
    pub name: String,
    pub passing: DartFFIParamPassing,
    pub write_seq: Option<WriteSeq>,
    pub ty: super::DartType,
}

impl DartFunctionParam {
    pub fn storage_name(&self) -> String {
        format!("l${}Storage", self.name)
    }

    pub fn bytes_name(&self) -> String {
        format!("l${}Bytes", self.name)
    }

    pub fn buf_writer_name(&self) -> String {
        format!("l${}Buf", self.name)
    }

    pub fn ffi_param_ptr_name(&self) -> String {
        format!("{}Ptr", self.name)
    }

    pub fn ffi_param_len_name(&self) -> String {
        format!("{}Len", self.name)
    }

    pub fn callable_name(&self) -> String {
        format!("l${}Callable", self.name)
    }

    pub fn wire_write_expr(&self) -> String {
        emit::emit_writer_write(
            self.write_seq.as_ref().expect("wire encoded"),
            &self.buf_writer_name(),
            &self.name,
        )
    }

    pub fn wire_size_expr(&self) -> String {
        let w = self.write_seq.as_ref().expect("wire encoded");
        emit::emit_size_expr(&w.size)
    }

    pub fn bytes_write_expr(&self) -> String {
        let DartFFIParamPassing::Bytes(bytes) = &self.passing else {
            panic!("bytes passsing")
        };

        let write = match bytes {
            DartFFIParamBytes::Array(value) => match value {
                DartFFIParamValue::Primitive(..) => {
                    format!("{}.writeBytes", self.buf_writer_name())
                }
                DartFFIParamValue::Record(record) => {
                    format!("{}._m$blittableWriteList", record)
                }
                DartFFIParamValue::Enum => String::new(),
            },
            DartFFIParamBytes::Record(record) => format!("{}._m$blittableWrite", record),
            DartFFIParamBytes::UTF8 => {
                format!("{}.writeBytes", self.buf_writer_name())
            }
        };

        let args = match bytes {
            DartFFIParamBytes::Array(value) => match value {
                DartFFIParamValue::Primitive(primitive) => match primitive {
                    super::DartFFIPrimitiveType::Bool => {
                        vec![format!("{}._bytes", self.name), String::from("0")]
                    }
                    super::DartFFIPrimitiveType::Int(..)
                    | super::DartFFIPrimitiveType::Float(..) => {
                        vec![self.bytes_name(), String::from("0")]
                    }
                },
                DartFFIParamValue::Record(..) => vec![self.name.clone(), self.buf_writer_name()],
                DartFFIParamValue::Enum => vec![format!("{}.value", self.name), String::from("0")],
            },
            DartFFIParamBytes::Record(..) => vec![self.name.clone(), self.buf_writer_name()],
            DartFFIParamBytes::UTF8 => vec![self.bytes_name(), String::from("0")],
        };

        format!(
            "{}({})",
            write,
            args.into_iter()
                .reduce(|acc, s| acc + ", " + s.as_str())
                .unwrap()
        )
    }

    pub fn bytes_create_expr(&self) -> Option<String> {
        let DartFFIParamPassing::Bytes(bytes) = &self.passing else {
            panic!("bytes passsing")
        };

        let create = match bytes {
            DartFFIParamBytes::Array(value) => match value {
                DartFFIParamValue::Primitive(primitive) => match primitive {
                    super::DartFFIPrimitiveType::Bool => "$$BoltBoolList.fromList".to_string(),
                    super::DartFFIPrimitiveType::Int(int) => match int {
                        super::DartFFIIntType::Uint8 => {
                            "$$typed_data.Uint8List.fromList".to_string()
                        }
                        super::DartFFIIntType::Int8 => "$$typed_data.Int8List.fromList".to_string(),
                        super::DartFFIIntType::Uint16 => {
                            "$$typed_data.Uint16List.fromList".to_string()
                        }
                        super::DartFFIIntType::Int16 => {
                            "$$typed_data.Int16List.fromList".to_string()
                        }
                        super::DartFFIIntType::Uint32 => {
                            "$$typed_data.Uint32List.fromList".to_string()
                        }
                        super::DartFFIIntType::Int32 => {
                            "$$typed_data.Int32List.fromList".to_string()
                        }
                        super::DartFFIIntType::Uint64 | super::DartFFIIntType::UintPtr => {
                            "$$typed_data.Uint64List.fromList".to_string()
                        }
                        super::DartFFIIntType::Int64 | super::DartFFIIntType::IntPtr => {
                            "$$typed_data.Int64List.fromList".to_string()
                        }
                    },
                    super::DartFFIPrimitiveType::Float(float) => match float {
                        super::DartFFIFloatType::Float32 => {
                            "$$typed_data.Float32List.fromList".to_string()
                        }
                        super::DartFFIFloatType::Float64 => {
                            "$$typed_data.Float64List.fromList".to_string()
                        }
                    },
                },
                DartFFIParamValue::Record(..) => return None,
                DartFFIParamValue::Enum => String::new(),
            },
            DartFFIParamBytes::Record(..) => return None,
            DartFFIParamBytes::UTF8 => "$$convert.utf8.encode".to_string(),
        };

        let var = match bytes {
            DartFFIParamBytes::Array(value) => match value {
                DartFFIParamValue::Primitive(primitive) => match primitive {
                    super::DartFFIPrimitiveType::Bool
                    | super::DartFFIPrimitiveType::Int(..)
                    | super::DartFFIPrimitiveType::Float(..) => self.name.clone(),
                },
                DartFFIParamValue::Record(record) => {
                    format!("{}, {}._k$structSize", self.name, record)
                }
                DartFFIParamValue::Enum => format!("{}.value", self.name),
            },
            DartFFIParamBytes::Record(record) => format!("{}, {}._k$structSize", self.name, record),
            DartFFIParamBytes::UTF8 => self.name.clone(),
        };

        Some(format!("{}({})", create, var))
    }

    pub fn bytes_len_expr(&self) -> String {
        let DartFFIParamPassing::Bytes(bytes) = &self.passing else {
            panic!("bytes passsing")
        };

        match bytes {
            DartFFIParamBytes::Array(value) => match value {
                DartFFIParamValue::Primitive(..) => {
                    format!("{}.lengthInBytes", self.bytes_name())
                }
                DartFFIParamValue::Record(record) => {
                    format!("{}.length * {}._k$structSize", self.name, record)
                }
                DartFFIParamValue::Enum => String::new(),
            },
            DartFFIParamBytes::Record(record) => {
                format!("{}.length * {}._k$structSize", self.name, record)
            }
            DartFFIParamBytes::UTF8 => format!("{}.lengthInBytes", self.bytes_name()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DartFFIAsyncFunctionDef {
    pub poll_symbol: String,
    pub complete_symbol: String,
    pub complete_ty: super::DartFFIType,
    pub cancel_symbol: String,
    pub free_symbol: String,
}

#[derive(Debug, Clone)]
pub enum DartFunctionMode {
    Sync,
    Async(DartFFIAsyncFunctionDef),
}

#[derive(Debug, Clone)]
pub struct DartFunction {
    pub mode: DartFunctionMode,
    pub ty: DartFunctionType,
    pub ffi_def: DartFFIFunctionDef,
    pub sig: super::DartFunctionSig,
    pub params: Vec<DartFunctionParam>,
    pub returns: super::DartType,
}

impl DartFunction {
    pub fn is_async(&self) -> bool {
        matches!(self.mode, DartFunctionMode::Async { .. })
    }

    pub fn self_storage_name(&self) -> String {
        String::from("l$$selfStorage")
    }

    pub fn self_wire_name(&self) -> String {
        String::from("l$$selfWire")
    }

    pub fn self_wire_size(&self) -> Option<String> {
        match &self.ty {
            DartFunctionType::TopLevel { .. } | DartFunctionType::Constructor { .. } => None,
            DartFunctionType::Method {
                receiver, owner, ..
            } => {
                match receiver {
                    DartMethodReceiver::Static => {
                        return None;
                    }
                    DartMethodReceiver::ReceiverPassing(passing) => {
                        if !matches!(passing, DartFFIParamPassing::WireEncoded) {
                            return None;
                        }
                    }
                }

                match owner {
                    DartFunctionCallOwner::Class(..) => None,
                    DartFunctionCallOwner::Record(..) | DartFunctionCallOwner::Enum(..) => {
                        Some(String::from("_m$wireEncodedSize()"))
                    }
                }
            }
        }
    }

    pub fn self_wire_encode_expr(&self) -> Option<String> {
        match &self.ty {
            DartFunctionType::TopLevel { .. } | DartFunctionType::Constructor { .. } => None,
            DartFunctionType::Method {
                receiver, owner, ..
            } => {
                match receiver {
                    DartMethodReceiver::Static => {
                        return None;
                    }
                    DartMethodReceiver::ReceiverPassing(passing) => {
                        if !matches!(passing, DartFFIParamPassing::WireEncoded) {
                            return None;
                        }
                    }
                }

                match owner {
                    DartFunctionCallOwner::Class(..) => None,
                    DartFunctionCallOwner::Record(..) | DartFunctionCallOwner::Enum(..) => {
                        Some(format!("_m$wireEncode({})", self.self_wire_name()))
                    }
                }
            }
        }
    }

    pub fn get_ffi_params(&self) -> impl Iterator<Item = String> {
        std::iter::chain(
            match &self.ty {
                DartFunctionType::TopLevel { .. } => vec![],
                DartFunctionType::Method {
                    receiver, owner, ..
                } => match receiver {
                    DartMethodReceiver::Static => vec![],
                    DartMethodReceiver::ReceiverPassing(recv_passing) => match recv_passing {
                        DartFFIParamPassing::Value(value) => match value {
                            DartFFIParamValue::Primitive(..) => match owner {
                                DartFunctionCallOwner::Class(..) => unreachable!(),
                                DartFunctionCallOwner::Record(..) => unreachable!(),
                                DartFunctionCallOwner::Enum(..) => vec![String::from("this.value")],
                            },
                            DartFFIParamValue::Record(_) => vec![String::from("_m$toStruct()")],
                            DartFFIParamValue::Enum => vec![String::from("this.value")],
                        },
                        DartFFIParamPassing::WireEncoded => vec![
                            format!("{}.ptr", self.self_storage_name()),
                            format!("{}.len", self.self_wire_name()),
                        ],
                        DartFFIParamPassing::Bytes(..) => vec![],
                        DartFFIParamPassing::Closure(..) => unreachable!(),
                        DartFFIParamPassing::ClassHandle => vec![String::from("_handle")],
                        DartFFIParamPassing::CallbackHandle { .. } => unreachable!(),
                    },
                },
                DartFunctionType::Constructor { .. } => vec![],
            },
            self.params.iter().flat_map(|p| match &p.passing {
                DartFFIParamPassing::Value(value) => match value {
                    DartFFIParamValue::Primitive(..) => match &p.ty {
                        super::DartType::Bool | super::DartType::Int | super::DartType::Double => {
                            vec![p.name.clone()]
                        }
                        super::DartType::Enum(_) => vec![format!("{}.value", p.name)],
                        super::DartType::Custom(_) => todo!(),
                        _ => unreachable!(),
                    },
                    DartFFIParamValue::Record(_) => vec![format!("{}._m$toStruct()", p.name)],
                    DartFFIParamValue::Enum => vec![format!("{}.value", p.name)],
                },
                DartFFIParamPassing::WireEncoded => {
                    vec![
                        format!("{}.ptr", p.storage_name()),
                        format!("{}.len", p.buf_writer_name()),
                    ]
                }
                DartFFIParamPassing::Bytes(bytes) => match bytes {
                    DartFFIParamBytes::Array(value) => match value {
                        DartFFIParamValue::Primitive(..) | DartFFIParamValue::Enum => vec![
                            format!("{}.ptr.cast()", p.storage_name()),
                            format!("{}.length", p.bytes_name()),
                        ],
                        DartFFIParamValue::Record(_) => vec![
                            format!("{}.ptr.cast()", p.storage_name()),
                            format!("{}.length", p.name),
                        ],
                    },
                    DartFFIParamBytes::Record(record) => vec![
                        format!("{}.ptr.cast()", p.storage_name()),
                        format!("{}._k$structSize", record),
                    ],
                    DartFFIParamBytes::UTF8 => vec![
                        format!("{}.ptr.cast()", p.storage_name()),
                        format!("{}.lengthInBytes", p.bytes_name()),
                    ],
                },
                DartFFIParamPassing::Closure(..) => {
                    vec![
                        format!("{}.nativeFunction", p.callable_name()),
                        String::from("$$ffi.nullptr"),
                    ]
                }
                DartFFIParamPassing::ClassHandle => vec![format!("{}._handle", p.name)],
                DartFFIParamPassing::CallbackHandle{ class, nullable } => if *nullable {
                    vec![format!("({name} == null) ? _$$BoltCallbackHandle.kNull : _I${class}.createCallbackHandle({name})", name = p.name)]
                } else {
                    vec![format!("_I${}.createCallbackHandle({})", class, p.name)]
                }
            }),
        )
    }
}

#[derive(Debug, Clone)]
pub enum DartFunctionCallOwner {
    Class(ClassId),
    Record(RecordId),
    Enum(EnumId),
}

impl DartFunctionCallOwner {
    pub fn name(&self) -> &str {
        match self {
            DartFunctionCallOwner::Class(s) => s.as_str(),
            DartFunctionCallOwner::Record(s) => s.as_str(),
            DartFunctionCallOwner::Enum(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DartConstructorKind {
    Default,
    Named { name: String },
}

#[derive(Debug, Clone)]
pub enum DartFunctionType {
    TopLevel {
        name: String,
    },
    Method {
        name: String,
        receiver: DartMethodReceiver,
        owner: DartFunctionCallOwner,
    },
    Constructor {
        kind: super::DartConstructorKind,
        is_fallible: bool,
        owner: DartFunctionCallOwner,
    },
}

#[derive(Debug, Clone)]
pub enum DartMethodReceiver {
    Static,
    ReceiverPassing(DartFFIParamPassing),
}

#[derive(Debug, Clone)]
pub struct DartFFIFunctionDef {
    pub sig: super::DartFFIFunctionSig,
    pub symbol: String,
    pub is_leaf: bool,
}
