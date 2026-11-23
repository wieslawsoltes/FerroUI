//! The program in memory before anything is measured.
//!
//! The system maps the file of a program and reads a page of it from the disk when the process
//! first touches it. A scene's build and show are measured once a process, so the code they run
//! for the first time is read inside what they time whenever the system does not hold those pages:
//! after another large build or process pushed them out, and always for a file that was just
//! written. The read is of the disk, not of the framework: the `list` scene, built and shown in
//! 1.1 and 1.7 ms with the file in memory, took 1.6 and 2.3 ms with about 1,150 pages read in
//! between, and the larger of two programs paid more of it.
//!
//! [`page_in_the_program`] touches the program once, ahead of the measurement.

/// Reads one byte of every page of the program's own file that the process has mapped (its code
/// and its constant and initialised data), so that no page of it is read from the disk inside a
/// measurement. Answers with the number of bytes it went over; zero where the platform gives no
/// way to find them.
///
/// The libraries of the system are not touched: they are shared with every process and in memory.
pub fn page_in_the_program() -> usize {
    image::page_in()
}

#[cfg(all(target_os = "macos", target_pointer_width = "64"))]
mod image {
    /// `mach_header_64` of `<mach-o/loader.h>`.
    #[repr(C)]
    struct Header {
        magic: u32,
        cpu_type: i32,
        cpu_subtype: i32,
        file_type: u32,
        command_count: u32,
        commands_size: u32,
        flags: u32,
        reserved: u32,
    }

    /// `load_command`: what every command starts with.
    #[repr(C)]
    struct Command {
        command: u32,
        size: u32,
    }

    /// `segment_command_64`.
    #[repr(C)]
    struct Segment {
        command: u32,
        size: u32,
        name: [u8; 16],
        address: u64,
        memory_size: u64,
        file_offset: u64,
        file_size: u64,
        maximum_protection: i32,
        initial_protection: i32,
        section_count: u32,
        flags: u32,
    }

    const MAGIC_64: u32 = 0xfeed_facf;
    const EXECUTABLE: u32 = 2;
    const SEGMENT_64: u32 = 0x19;
    const READABLE: i32 = 1;
    /// The symbols and the signature of the program: the loader reads what it needs of them, and
    /// nothing a scene runs does.
    const LINK_EDIT: &[u8] = b"__LINKEDIT";
    /// Smaller than a page of any Mac (4 KB on Intel, 16 KB on Apple silicon).
    const STEP: usize = 4096;

    extern "C" {
        fn _dyld_image_count() -> u32;
        fn _dyld_get_image_header(index: u32) -> *const Header;
        fn _dyld_get_image_vmaddr_slide(index: u32) -> isize;
    }

    pub(super) fn page_in() -> usize {
        // SAFETY: the loader's own list of the images of this process; an index below the count
        // answers with the header the loader mapped, or with null.
        let program = (0..unsafe { _dyld_image_count() }).find_map(|index| {
            let header = unsafe { _dyld_get_image_header(index) };
            if header.is_null() {
                return None;
            }
            // SAFETY: a header the loader mapped is readable for its whole size.
            let found = unsafe { (*header).magic == MAGIC_64 && (*header).file_type == EXECUTABLE };
            found.then(|| (header, unsafe { _dyld_get_image_vmaddr_slide(index) }))
        });
        let Some((header, slide)) = program else { return 0 };

        let mut bytes = 0;
        // SAFETY: the load commands follow the header, `command_count` of them, each as long as
        // its `size` says; the loader validated them when it mapped the program. A segment that
        // is readable and backed by the file is mapped at its address plus the slide for at least
        // its size in the file, and the program stays mapped for the life of the process.
        unsafe {
            let mut command = header.add(1).cast::<Command>();
            for _ in 0..(*header).command_count {
                if (*command).command == SEGMENT_64 {
                    let segment = &*command.cast::<Segment>();
                    let length = segment.file_size.min(segment.memory_size) as usize;
                    let name = segment.name.split(|&byte| byte == 0).next().unwrap_or_default();
                    if length != 0 && segment.initial_protection & READABLE != 0 && name != LINK_EDIT {
                        let start = (segment.address as isize + slide) as *const u8;
                        let mut offset = 0;
                        while offset < length {
                            std::hint::black_box(start.add(offset).read_volatile());
                            offset += STEP;
                        }
                        bytes += length;
                    }
                }
                command = command.cast::<u8>().add((*command).size as usize).cast::<Command>();
            }
        }
        bytes
    }
}

#[cfg(not(all(target_os = "macos", target_pointer_width = "64")))]
mod image {
    /// Reads the file of the program: the system then holds its pages, and a page the process
    /// touches for the first time is mapped without a read of the disk.
    pub(super) fn page_in() -> usize {
        std::env::current_exe().and_then(std::fs::read).map_or(0, |file| std::hint::black_box(file).len())
    }
}
