//! A card, its resources, connectors, encoders and modes as values (the
//! port of `Output/DrmBindings.cs`).

use super::drm::lib_drm::{drmModeConnector, drmModeCrtc, drmModeEncoder, drmModeModeInfo, DrmModeType};
use super::drm::{DrmModeConnection, DrmModeSubPixel};
use crate::DrmConnectorType;
use ferroui_base::{PixelSize, Size};
use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::atomic::{AtomicI32, Ordering};

static KNOWN_CONNECTOR_TYPES: [&str; 17] = [
    "None",
    "VGA",
    "DVI-I",
    "DVI-D",
    "DVI-A",
    "Composite",
    "S-Video",
    "LVDS",
    "Component",
    "DIN",
    "DisplayPort",
    "HDMI-A",
    "HDMI-B",
    "TV",
    "eDP",
    "Virtual",
    "DSI",
];

/// A connector of a card, as it was when the resources were read.
#[derive(Clone, Debug)]
pub struct DrmConnector {
    connection: DrmModeConnection,
    id: u32,
    name: String,
    size_mm: Size,
    sub_pixel: DrmModeSubPixel,
    encoder_id: u32,
    encoder_ids: Vec<u32>,
    modes: Vec<DrmModeInfo>,
    connector_type: DrmConnectorType,
    connector_type_id: u32,
}

impl DrmConnector {
    /// The connector the library describes.
    ///
    /// # Safety
    /// `conn` is a connector of the library: its arrays of modes and
    /// encoders hold the numbers of elements its counts say.
    pub(crate) unsafe fn new(conn: &drmModeConnector) -> Self {
        let mut encoder_ids = Vec::new();
        for c in 0..conn.count_encoders.max(0) as usize {
            // SAFETY: the array has `count_encoders` elements (the contract of the function).
            encoder_ids.push(unsafe { *conn.encoders.add(c) });
        }
        let mut modes = Vec::new();
        for c in 0..conn.count_modes.max(0) as usize {
            // SAFETY: the array has `count_modes` elements (the contract of the function).
            modes.push(DrmModeInfo::new(unsafe { &*conn.modes.add(c) }));
        }
        let name = if conn.connector_type as usize > KNOWN_CONNECTOR_TYPES.len() - 1 {
            format!("Unknown({})-{}", conn.connector_type, conn.connector_type_id)
        } else {
            format!("{}-{}", KNOWN_CONNECTOR_TYPES[conn.connector_type as usize], conn.connector_type_id)
        };
        Self {
            connection: conn.connection,
            id: conn.connector_id,
            size_mm: Size::new(conn.mm_width as f64, conn.mm_height as f64),
            sub_pixel: conn.subpixel,
            encoder_ids,
            encoder_id: conn.encoder_id,
            modes,
            connector_type: DrmConnectorType(conn.connector_type),
            connector_type_id: conn.connector_type_id,
            name,
        }
    }

    pub fn connection(&self) -> DrmModeConnection {
        self.connection
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size_mm(&self) -> Size {
        self.size_mm
    }

    pub fn sub_pixel(&self) -> DrmModeSubPixel {
        self.sub_pixel
    }

    pub(crate) fn encoder_id(&self) -> u32 {
        self.encoder_id
    }

    pub(crate) fn encoder_ids(&self) -> &[u32] {
        &self.encoder_ids
    }

    pub fn modes(&self) -> &[DrmModeInfo] {
        &self.modes
    }

    pub fn connector_type(&self) -> DrmConnectorType {
        self.connector_type
    }

    pub fn connector_type_id(&self) -> u32 {
        self.connector_type_id
    }
}

/// A display mode of a connector.
#[derive(Clone, Debug)]
pub struct DrmModeInfo {
    pub(crate) mode: drmModeModeInfo,
    name: Option<String>,
}

impl DrmModeInfo {
    pub(crate) fn new(info: &drmModeModeInfo) -> Self {
        // The name ends at its first zero, or with the array.
        let bytes: Vec<u8> = info.name.iter().map(|&c| c as u8).collect();
        let name = match CStr::from_bytes_until_nul(&bytes) {
            Ok(name) => name.to_string_lossy().into_owned(),
            Err(_) => String::from_utf8_lossy(&bytes).into_owned(),
        };
        Self { mode: *info, name: Some(name) }
    }

    pub fn resolution(&self) -> PixelSize {
        PixelSize::new(self.mode.hdisplay as i32, self.mode.vdisplay as i32)
    }

    pub fn is_preferred(&self) -> bool {
        self.mode.type_.has_all_flags(DrmModeType::DRM_MODE_TYPE_PREFERRED)
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// An encoder of a card and the CRTCs it can drive.
#[derive(Clone, Debug)]
pub(crate) struct DrmEncoder {
    encoder: drmModeEncoder,
    possible_crtcs: Vec<drmModeCrtc>,
}

impl DrmEncoder {
    pub fn new(encoder: drmModeEncoder, crtcs: &[drmModeCrtc]) -> Self {
        let mut possible_crtcs = Vec::new();
        for (c, crtc) in crtcs.iter().enumerate() {
            // A card has at most 32 CRTCs: the bits of `possible_crtcs`.
            let bit = 1u32.checked_shl(c as u32).unwrap_or(0);
            if (encoder.possible_crtcs & bit) != 0 {
                possible_crtcs.push(*crtc);
            }
        }
        Self { encoder, possible_crtcs }
    }

    pub fn encoder(&self) -> drmModeEncoder {
        self.encoder
    }

    pub fn possible_crtcs(&self) -> &[drmModeCrtc] {
        &self.possible_crtcs
    }
}

/// The connectors and encoders of a card.
#[derive(Clone, Debug, Default)]
pub struct DrmResources {
    connectors: Vec<DrmConnector>,
    encoders: HashMap<u32, DrmEncoder>,
}

impl DrmResources {
    /// Reads the resources of the card `fd`; with `connectors_force_probe`
    /// the connectors are probed (`drmModeGetConnector`, which may take
    /// long), else their current state is read.
    #[cfg(target_os = "linux")]
    pub fn new(fd: i32, connectors_force_probe: bool) -> std::io::Result<Self> {
        use super::drm::lib_drm::LibDrm;
        use std::io;

        let lib = LibDrm::get()?;
        // SAFETY: a call with a descriptor; the result is null or resources the caller frees.
        let res = unsafe { lib.drm_mode_get_resources(fd) };
        if res.is_null() {
            return Err(io::Error::new(
                io::Error::last_os_error().kind(),
                format!("drmModeGetResources failed: {}", io::Error::last_os_error()),
            ));
        }

        let read = || -> io::Result<Self> {
            // SAFETY: `res` is the resources of the card, alive until they are freed below; its
            // arrays have the numbers of elements its counts say.
            let res = unsafe { &*res };
            let mut this = Self::default();
            let mut crtcs = Vec::with_capacity(res.count_crtcs.max(0) as usize);
            for c in 0..res.count_crtcs.max(0) as usize {
                // SAFETY: see above; the result is null or a CRTC the caller frees.
                let crtc = unsafe { lib.drm_mode_get_crtc(fd, *res.crtcs.add(c)) };
                if crtc.is_null() {
                    return Err(io::Error::other("drmModeGetCrtc failed"));
                }
                // SAFETY: a CRTC of the library, read and freed once.
                unsafe {
                    crtcs.push(*crtc);
                    lib.drm_mode_free_crtc(crtc);
                }
            }

            for c in 0..res.count_encoders.max(0) as usize {
                // SAFETY: as for the CRTCs.
                let id = unsafe { *res.encoders.add(c) };
                // SAFETY: as for the CRTCs.
                let enc = unsafe { lib.drm_mode_get_encoder(fd, id) };
                if enc.is_null() {
                    return Err(io::Error::other("drmModeGetEncoder failed"));
                }
                // SAFETY: an encoder of the library, read and freed once.
                unsafe {
                    this.encoders.insert(id, DrmEncoder::new(*enc, &crtcs));
                    lib.drm_mode_free_encoder(enc);
                }
            }

            for c in 0..res.count_connectors.max(0) as usize {
                // SAFETY: as for the CRTCs.
                let id = unsafe { *res.connectors.add(c) };
                // SAFETY: as for the CRTCs.
                let conn = unsafe {
                    if connectors_force_probe {
                        lib.drm_mode_get_connector(fd, id)
                    } else {
                        lib.drm_mode_get_connector_current(fd, id)
                    }
                };
                if conn.is_null() {
                    return Err(io::Error::other("drmModeGetConnector failed"));
                }
                // SAFETY: a connector of the library, read and freed once.
                unsafe {
                    this.connectors.push(DrmConnector::new(&*conn));
                    lib.drm_mode_free_connector(conn);
                }
            }
            Ok(this)
        };
        let result = read();
        // SAFETY: the resources the call above returned, freed once.
        unsafe { lib.drm_mode_free_resources(res) };
        result
    }

    /// The resources made of values, for the tests of the selection of the
    /// connector, the mode and the CRTC without a card.
    #[cfg(test)]
    pub(crate) fn from_parts(connectors: Vec<DrmConnector>, encoders: HashMap<u32, DrmEncoder>) -> Self {
        Self { connectors, encoders }
    }

    pub fn connectors(&self) -> &[DrmConnector] {
        &self.connectors
    }

    pub(crate) fn encoders(&self) -> &HashMap<u32, DrmEncoder> {
        &self.encoders
    }

    /// Writes the connectors and their modes to the standard output.
    /// (Internal in the reference, where nothing calls it; public here for
    /// the diagnostics of the smoke example.)
    pub fn dump(&self) {
        fn print(off: usize, s: &str) {
            println!("{}{s}", "    ".repeat(off));
        }
        print(0, "Connectors");
        for conn in &self.connectors {
            print(1, &format!("{}:", conn.name()));
            print(2, &format!("Id: {}", conn.id()));
            print(2, &format!("Size: {} mm", conn.size_mm()));
            print(2, &format!("Encoder id: {}", conn.encoder_id()));
            print(2, "Modes");
            for m in conn.modes() {
                print(3, &format!("{} {}", m.name().unwrap_or_default(), if m.is_preferred() { "PREFERRED" } else { "" }));
            }
        }
    }
}

/// A card of the DRM subsystem, open for reading and writing.
#[derive(Debug)]
pub struct DrmCard {
    fd: AtomicI32,
}

impl DrmCard {
    /// Opens the card `path`, or, without one, the first card of
    /// `/dev/dri` (by its number) that has CRTCs, encoders and connectors.
    #[cfg(target_os = "linux")]
    pub fn new(path: Option<&str>) -> std::io::Result<Self> {
        use super::drm::lib_drm::LibDrm;
        use std::collections::BTreeMap;
        use std::ffi::CString;
        use std::io;

        let open = |file: &str| -> i32 {
            let Ok(file) = CString::new(file) else {
                return -1;
            };
            // SAFETY: a valid C string.
            unsafe { libc::open(file.as_ptr(), libc::O_RDWR, 0) }
        };

        match path {
            None => {
                let lib = LibDrm::get()?;
                let mut files = BTreeMap::new();
                if let Ok(entries) = std::fs::read_dir("/dev/dri/") {
                    for entry in entries.flatten() {
                        let file_name = entry.file_name().to_string_lossy().into_owned();
                        let Some(number) = file_name.strip_prefix("card") else {
                            continue;
                        };
                        // Digits only (`NumberStyles.None`).
                        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
                            continue;
                        }
                        if let Ok(card_number) = number.parse::<u32>() {
                            files.insert(card_number, entry.path().to_string_lossy().into_owned());
                        }
                    }
                }

                for file in files.values() {
                    let fd = open(file);
                    if fd == -1 {
                        continue;
                    }

                    // SAFETY: a call with a descriptor; the result is null or resources that
                    // are read here and freed once.
                    unsafe {
                        let resources = lib.drm_mode_get_resources(fd);
                        if !resources.is_null()
                            && (*resources).count_crtcs > 0
                            && (*resources).count_encoders > 0
                            && (*resources).count_connectors > 0
                        {
                            lib.drm_mode_free_resources(resources);
                            return Ok(Self { fd: AtomicI32::new(fd) });
                        }

                        if !resources.is_null() {
                            lib.drm_mode_free_resources(resources);
                        }
                        libc::close(fd);
                    }
                }

                Err(io::Error::new(io::ErrorKind::NotFound, "Couldn't open /dev/dri/card[0-9]+"))
            }
            Some(path) => {
                let fd = open(path);
                if fd == -1 {
                    let error = io::Error::last_os_error();
                    return Err(io::Error::new(error.kind(), format!("Couldn't open {path}: {error}")));
                }
                Ok(Self { fd: AtomicI32::new(fd) })
            }
        }
    }

    /// The card of a descriptor that is already open; the card closes it.
    pub fn from_fd(fd: i32) -> Self {
        Self { fd: AtomicI32::new(fd) }
    }

    /// The descriptor of the card, or -1 once it is disposed.
    pub fn fd(&self) -> i32 {
        self.fd.load(Ordering::Acquire)
    }

    #[cfg(target_os = "linux")]
    pub fn get_resources(&self, connectors_force_probe: bool) -> std::io::Result<DrmResources> {
        DrmResources::new(self.fd(), connectors_force_probe)
    }

    /// Closes the card.
    pub fn dispose(&self) {
        let fd = self.fd.swap(-1, Ordering::AcqRel);
        if fd == -1 {
            return;
        }
        // SAFETY: the descriptor the card owns, closed once.
        unsafe {
            libc::close(fd);
        }
    }
}

impl Drop for DrmCard {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    /// A mode of `width` by `height`, preferred or not, named as the
    /// kernel names modes.
    pub(crate) fn mode(width: u16, height: u16, preferred: bool) -> drmModeModeInfo {
        let mut info = drmModeModeInfo { hdisplay: width, vdisplay: height, ..Default::default() };
        if preferred {
            info.type_ = DrmModeType::DRM_MODE_TYPE_PREFERRED | DrmModeType::DRM_MODE_TYPE_DRIVER;
        }
        for (target, source) in info.name.iter_mut().zip(format!("{width}x{height}").bytes()) {
            *target = source as _;
        }
        info
    }

    /// A connector of the library, with its arrays.
    pub(crate) fn connector(
        id: u32,
        connector_type: u32,
        connector_type_id: u32,
        connection: DrmModeConnection,
        encoder_id: u32,
        encoders: &[u32],
        modes: &[drmModeModeInfo],
    ) -> DrmConnector {
        let mut encoders = encoders.to_vec();
        let mut modes = modes.to_vec();
        let raw = drmModeConnector {
            connector_id: id,
            encoder_id,
            connector_type,
            connector_type_id,
            connection,
            mm_width: 300,
            mm_height: 200,
            subpixel: DrmModeSubPixel::DRM_MODE_SUBPIXEL_UNKNOWN,
            count_modes: modes.len() as i32,
            modes: modes.as_mut_ptr(),
            count_props: 0,
            props: std::ptr::null_mut(),
            prop_values: std::ptr::null_mut(),
            count_encoders: encoders.len() as i32,
            encoders: encoders.as_mut_ptr(),
        };
        // SAFETY: the arrays are the two vectors, with the counts of their lengths.
        unsafe { DrmConnector::new(&raw) }
    }

    #[test]
    fn a_connector_is_named_after_its_type_and_reads_its_arrays() {
        let conn = connector(
            31,
            15,
            1,
            DrmModeConnection::DRM_MODE_CONNECTED,
            40,
            &[40, 41],
            &[mode(1024, 768, true), mode(800, 600, false)],
        );
        assert_eq!(conn.name(), "Virtual-1");
        assert_eq!(conn.id(), 31);
        assert_eq!(conn.connector_type(), DrmConnectorType::VIRTUAL);
        assert_eq!(conn.connector_type_id(), 1);
        assert_eq!(conn.connection(), DrmModeConnection::DRM_MODE_CONNECTED);
        assert_eq!(conn.size_mm(), Size::new(300.0, 200.0));
        assert_eq!(conn.encoder_id(), 40);
        assert_eq!(conn.encoder_ids(), [40, 41]);
        assert_eq!(conn.modes().len(), 2);
        assert_eq!(conn.modes()[0].name(), Some("1024x768"));
        assert_eq!(conn.modes()[0].resolution(), PixelSize::new(1024, 768));
        assert!(conn.modes()[0].is_preferred());
        assert!(!conn.modes()[1].is_preferred());

        let hdmi = connector(32, 11, 2, DrmModeConnection::DRM_MODE_DISCONNECTED, 0, &[], &[]);
        assert_eq!(hdmi.name(), "HDMI-A-2");
        assert!(hdmi.modes().is_empty());
        // Types the table does not know (a writeback connector is 18).
        let unknown = connector(33, 18, 1, DrmModeConnection::DRM_MODE_UNKNOWNCONNECTION, 0, &[], &[]);
        assert_eq!(unknown.name(), "Unknown(18)-1");
        assert_eq!(unknown.connector_type(), DrmConnectorType(18));
    }

    #[test]
    fn a_name_of_32_characters_without_its_zero_is_read_whole() {
        let mut info = drmModeModeInfo::default();
        info.name = [b'a' as _; 32];
        assert_eq!(DrmModeInfo::new(&info).name().map(str::len), Some(32));
    }

    #[test]
    fn an_encoder_lists_the_crtcs_of_its_bits() {
        let crtcs: Vec<drmModeCrtc> =
            (0..3).map(|c| drmModeCrtc { crtc_id: 100 + c, ..Default::default() }).collect();
        let encoder = DrmEncoder::new(drmModeEncoder { encoder_id: 7, possible_crtcs: 0b101, ..Default::default() }, &crtcs);
        assert_eq!(encoder.encoder().encoder_id, 7);
        let ids: Vec<u32> = encoder.possible_crtcs().iter().map(|crtc| crtc.crtc_id).collect();
        assert_eq!(ids, [100, 102]);
    }

    #[test]
    fn a_card_closes_its_descriptor_once() {
        let mut fds = [0; 2];
        // SAFETY: an array of two descriptors.
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        let card = DrmCard::from_fd(fds[0]);
        assert_eq!(card.fd(), fds[0]);
        card.dispose();
        assert_eq!(card.fd(), -1);
        card.dispose();
        drop(card);
        // SAFETY: the other end, which the card did not own.
        unsafe { libc::close(fds[1]) };
    }
}
