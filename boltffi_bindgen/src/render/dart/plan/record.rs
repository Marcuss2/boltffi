use crate::{
    ir::{PrimitiveType, ReadSeq, ValueExpr, WriteSeq},
    render::dart::emit,
};

#[derive(Debug, Clone)]
pub struct DartRecordField {
    pub name: String,
    pub offset: usize,
    pub ty: super::DartType,
    pub read_seq: ReadSeq,
    pub write_seq: WriteSeq,
}

impl DartRecordField {
    pub fn wire_decode_expr(&self, reader_name: &str) -> String {
        emit::emit_reader_read(&self.read_seq, reader_name, self.ty.is_inner_void())
    }

    pub fn wire_encode_expr(&self, writer_name: &str) -> String {
        emit::emit_writer_write(&self.write_seq, writer_name, &self.name)
    }

    pub fn wire_encoded_size_expr(&self) -> String {
        emit::emit_size_expr(&emit::remap_size_expr_value_expr(
            &self.write_seq.size,
            ValueExpr::Named(self.name.to_string()),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct DartBlittableLayout {
    pub struct_name: String,
    pub struct_size: usize,
    pub fields: Vec<DartBlittableField>,
}

#[derive(Debug, Clone)]
pub struct DartBlittableField {
    pub name: String,
    pub primitive: PrimitiveType,
    pub native_type: super::DartFFIType,
    pub offset_const_name: String,
    pub offset: usize,
}

impl DartBlittableField {
    pub fn primitive_write_method(&self) -> &'static str {
        emit::primitive_write_method(self.primitive)
    }

    pub fn primitive_read_method(&self) -> &'static str {
        emit::primitive_read_method(self.primitive)
    }

    pub fn blittable_read_expr(&self, bytes_name: &str) -> String {
        emit::emit_read_blittable_value(&self.offset_const_name, self.primitive, bytes_name)
    }

    pub fn blittable_write_expr(&self, bytes_name: &str) -> String {
        emit::emit_write_blittable_value(
            &self.offset_const_name,
            self.primitive,
            &self.name,
            bytes_name,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DartRecordInterface {
    Exception,
}

impl DartRecordInterface {
    pub fn name(&self) -> String {
        match self {
            DartRecordInterface::Exception => String::from("Exception"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DartRecord {
    pub name: String,
    pub interfaces: Vec<DartRecordInterface>,
    pub fields: Vec<DartRecordField>,
    pub blittable_layout: Option<DartBlittableLayout>,
    pub constructors: Vec<super::DartFunction>,
    pub methods: Vec<super::DartFunction>,
}
