use crate::{
    ir::{ReadSeq, StreamMode},
    render::dart::{DartFFIFunctionDef, DartFFIType, emit},
};

#[derive(Debug, Clone)]
pub struct DartStream {
    pub name: String,
    pub item_ty: super::DartType,
    pub item_read_seq: ReadSeq,
    pub ffi_item_ty: DartFFIType,
    pub ffi_item_size: Option<usize>,
    pub subscribe_fn: DartFFIFunctionDef,
    pub poll_fn: DartFFIFunctionDef,
    pub pop_batch_fn: DartFFIFunctionDef,
    pub wait_fn: DartFFIFunctionDef,
    pub unsubscribe_fn: DartFFIFunctionDef,
    pub free_fn: DartFFIFunctionDef,
    pub mode: StreamMode,
}

impl DartStream {
    pub fn item_wire_decode_expr(&self, reader_name: &str) -> String {
        emit::emit_reader_read(&self.item_read_seq, reader_name)
    }
}
