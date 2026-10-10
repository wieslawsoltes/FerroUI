//! OLE data transfer, the shell and the input pane: the COM interfaces
//! (generated from `win32.idl` by the MicroCom generator in the build
//! script of the crate).
//!
//! The bindings are values and vtable calls, so they compile on every host.

#![allow(non_camel_case_types)]

#[allow(
    clippy::all,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    missing_docs,
    unused_unsafe,
    unsafe_op_in_unsafe_fn
)]
mod win32 {
    include!(concat!(env!("OUT_DIR"), "/win32.rs"));
}

pub use win32::*;

#[cfg(test)]
mod tests {
    use crate::interop::unmanaged_methods::{DVASPECT, FORMATETC, STGMEDIUM, TYMED};

    /// The structures the interfaces of a data transfer pass have the
    /// layouts of the system headers.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn the_structures_of_a_data_transfer_have_the_sizes_of_the_system() {
        assert_eq!(std::mem::size_of::<FORMATETC>(), 32);
        assert_eq!(std::mem::offset_of!(FORMATETC, ptd), 8);
        assert_eq!(std::mem::offset_of!(FORMATETC, dw_aspect), 16);
        assert_eq!(std::mem::offset_of!(FORMATETC, lindex), 20);
        assert_eq!(std::mem::offset_of!(FORMATETC, tymed), 24);
        assert_eq!(std::mem::size_of::<STGMEDIUM>(), 24);
        assert_eq!(std::mem::offset_of!(STGMEDIUM, unionmember), 8);
        assert_eq!(std::mem::offset_of!(STGMEDIUM, p_unk_for_release), 16);
    }

    #[test]
    fn the_kinds_of_medium_and_the_aspects_have_the_values_of_the_system() {
        assert_eq!(TYMED::TYMED_HGLOBAL.bits(), 1);
        assert_eq!(TYMED::TYMED_ISTREAM.bits(), 4);
        assert_eq!(TYMED::TYMED_ENHMF.bits(), 64);
        assert_eq!(DVASPECT::DVASPECT_CONTENT.bits(), 1);
        assert_eq!(DVASPECT::DVASPECT_DOCPRINT.bits(), 8);
    }

    /// The interfaces of the file the stages 2d and 2e use are generated.
    #[test]
    fn the_interfaces_have_the_identifiers_of_the_system() {
        use super::{IDataObject, IDropTarget, IFileOpenDialog, IShellItem};
        use ferroui_microcom::{Guid, Interface};

        assert_eq!(IDataObject::IID, Guid::parse("0000010E-0000-0000-C000-000000000046").unwrap());
        assert_eq!(IDropTarget::IID, Guid::parse("00000122-0000-0000-C000-000000000046").unwrap());
        assert_eq!(IShellItem::IID, Guid::parse("43826D1E-E718-42EE-BC55-A1E261C37BFE").unwrap());
        assert_eq!(IFileOpenDialog::IID, Guid::parse("D57C7288-D4AD-4768-BE02-9D969532D960").unwrap());
    }
}
