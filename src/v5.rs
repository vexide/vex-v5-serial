//! Contains Structs and Enums that can contain metadata about the V5 System and Files stored on the V5 Robot Brain.
use bitflags::bitflags;

/// The epoch of the serial protocols timestamps
pub const J2000_EPOCH: u32 = 946684800;

#[derive(Copy, Clone, Debug)]
pub struct V5FirmwareVersion {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
    pub beta: u8,
}
impl V5FirmwareVersion {
    pub fn into_le_bytes(self) -> [u8; 4] {
        [self.beta, self.build, self.minor, self.major]
    }
}

/// Enum that represents the channel
/// for the V5 Controller
///
/// # Variants
///
/// * [V5ControllerChannel::Pit] - Used when controlling the robot outside of a competition match
/// * [V5ControllerChannel::Download] - Used when wirelessly uploading/downloading data to/from the V5 Brain
#[derive(Copy, Clone)]
pub enum V5ControllerChannel {
    /// Used when controlling the robot outside of a competition match
    Pit = 0x00,
    /// Used when wirelessly uploading or downloading data to/from the V5
    /// Brain
    Download = 0x01,
}

/// Enum that represents a vex product
///
/// # Variants
///
/// * [VexProductType::V5Brain] - Represents a V5 Robot Brain
/// * [VexProductType::V5Controller] - Represents a V5 Robot Controller
#[derive(Copy, Clone, Debug)]
pub enum VexProductType {
    /// Represents a V5 Robot Brain
    V5Brain(V5BrainFlags),
    /// Represents a V5 Robot Controller
    V5Controller(V5ControllerFlags),
}

impl From<VexProductType> for u8 {
    /// Converts the VexProductType to a u8 usable in the serial protocol.
    ///
    /// # Returns
    /// * [u8] where
    ///     * [VexProductType::V5Brain] == 0x10
    ///     * [VexProductType::V5Controller] == 0x11
    fn from(product: VexProductType) -> u8 {
        match product {
            VexProductType::V5Brain(_) => 0x10,
            VexProductType::V5Controller(_) => 0x11,
        }
    }
}

impl TryFrom<(u8, u8)> for VexProductType {
    type Error = crate::errors::DeviceError;
    /// Converts a tuple of two u8's into a Vex Product Type
    ///
    /// # Arguments
    /// * `0` - A [u8] value of either 0x10 or 0x11 which represents a [VexProductType::V5Brain] or a [VexProductType::V5Controller] respectively.
    /// * `1` - A [u8] that is parsed by [V5BrainFlags] and passed as a member of the [VexProductType] variant returned. If this parsing fails, the flags are all set to none.
    fn try_from(value: (u8, u8)) -> Result<VexProductType, Self::Error> {
        match value.0 {
            0x10 => Ok(VexProductType::V5Brain(
                V5BrainFlags::from_bits(value.1).unwrap_or(V5BrainFlags::empty()),
            )),
            0x11 => Ok(VexProductType::V5Controller(
                V5ControllerFlags::from_bits(value.1).unwrap_or(V5ControllerFlags::empty()),
            )),
            _ => Err(crate::errors::DeviceError::InvalidDevice),
        }
    }
}

bitflags! {
    /// Configuration flags for the v5 brain
    ///
    /// # Members
    /// * [V5BrainFlags::NONE] - There are no documented flags for the v5 brain. Testing will need to be done to determine the actual flags.
    #[derive(Debug, Clone, Copy)]
    pub struct V5BrainFlags: u8 {}
    /// Configuration flags for the v5 controller
    ///
    /// # Members
    /// * [V5ControllerFlags::NONE] - Represents that no flags are set
    /// * [V5ControllerFlags::CONNECTED_CABLE] - Bit 1 is set when the controller is connected over a cable to the V5 Brain
    /// * [V5ControllerFlags::CONNECTED_WIRELESS] - Bit 2 is set when the controller is connected over VEXLink to the V5 Brain.
    #[derive(Debug, Clone, Copy)]
    pub struct V5ControllerFlags: u8 {
        /// Bit 1 is set when the controller is connected over a cable to the V5 Brain
        const CONNECTED_CABLE = 1 << 0; // From testing, this appears to be how it works.
        /// Bit 2 is set when the controller is connected over VEXLink to the V5 Brain.
        const CONNECTED_WIRELESS = 1 << 1;
    }
}

// # File Transfer structures
// These structures are used during file transfers between the brain and the host

/// The function to be performed during the file transfer
///
/// # Variants
///
/// * [FileTransferFunction::Upload] - Specifies that a file is being uploaded/written to the brain
/// * [FileTransferFunction::Download] - Specifies that a file is being downloaded/read from the brain.
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum FileTransferFunction {
    /// Specifies that a file is being uploaded/written to the brain
    Upload = 0x01,
    /// Specifies that a file is being downloaded/read from the brain.
    Download = 0x02,
}

/// The target storage device of a file transfer
///
/// # Variants
///
/// * [FileTransferTarget::Flash] - The flash memory on the robot brain where most program files are stored
/// * [FileTransferTarget::Screen] - The memory accessed when taking a screen capture from the brain.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default)]
pub enum FileTransferTarget {
    Ddr = 0,
    /// Memory where programs are stored
    #[default]
    Qspi = 1,
    /// Screen capture memory
    Cbuf = 2,
    Vbuf = 3,
    Ddrc = 4,
    Ddre = 5,
    Flash = 6,
    Radio = 7,
    A1 = 13,
    B1 = 14,
    B2 = 15,
}

/// The VID of a file transfer
///
/// This appears to simply be metadata on what software wrote the file, however I am not entirely sure. To be safe, use User, as it appears to work.
///
/// # Variants
/// * [FileTransferVID::User]
/// * [FileTransferVID::System] - I am unsure what exactly User and System are intended to be used for, however vexrs uses the User variant when doing file operations, as it appears to work.
/// * [FileTransferVID::RMS] - The VID used by Robot Mesh Studio
/// * [FileTransferVID::PROS] - The VID used by Purdue Robotics Operating System
/// * [FileTransferVID::MW] - I am unsure which software uses the acronym MW, however this VID is used by it.
/// * [FileTransferVID::Other] - Allows specifying custom VIDs.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default)]
pub enum FileTransferVID {
    /// I am unsure what exactly User and System are intended to be used for, however vexrs uses the User variant when doing file operations, as it appears to work.
    #[default]
    User = 1,
    /// I am unsure what exactly User and System are intended to be used for, however vexrs uses the User variant when doing file operations, as it appears to work.
    System = 15,
    /// The VID used by Robot Mesh Studio
    RobotMesh = 16,
    /// The VID used by Purdue Robotics Operating System
    PROS = 24,
    /// I am unsure which software uses the acronym MW, however this VID is used by it.
    MW = 32,
    Vex = 240,
    /// Allows specifying custom VIDs.
    Other(u8),
}

impl FileTransferVID {
    /// Converts a [u8] to a [FileTransferVID]
    ///
    /// # Arguments
    /// * `v` - A [u8] where:
    ///     * `1`  == [FileTransferVID::User]
    ///     * `15` == [FileTransferVID::System]
    ///     * `16` == [FileTransferVID::RMS]
    ///     * `24` == [FileTransferVID::PROS]
    ///     * `32` == [FileTransferVID::MW]
    ///     * `_`  == [FileTransferVID::Other(_)]
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::User,
            15 => Self::System,
            16 => Self::RobotMesh,
            24 => Self::PROS,
            32 => Self::MW,
            a => Self::Other(a),
        }
    }

    /// Converts a [FileTransferVID] to a [u8]
    ///
    /// # Returns
    /// * A [u8] where:
    ///     * `1`  == [FileTransferVID::User]
    ///     * `15` == [FileTransferVID::System]
    ///     * `16` == [FileTransferVID::RMS]
    ///     * `24` == [FileTransferVID::PROS]
    ///     * `32` == [FileTransferVID::MW]
    ///     * `_`  == [FileTransferVID::Other(_)]
    pub fn to_u8(self) -> u8 {
        match self {
            FileTransferVID::User => 1,
            FileTransferVID::System => 15,
            FileTransferVID::RobotMesh => 16,
            FileTransferVID::PROS => 24,
            FileTransferVID::MW => 32,
            FileTransferVID::Vex => 240,
            FileTransferVID::Other(a) => a,
        }
    }
}

bitflags! {
    /// Options in a file transfer
    ///
    /// # Members
    /// * [FileTransferOptions::NONE] - Represents that no options are set
    /// * [FileTransferOptions::OVERWRITE] - Bit 1 is set when the file should be overwritten by the current operation.
    #[derive(Clone, Copy, Debug)]
    pub struct FileTransferOptions: u8 {
        /// Bit 1 is set when the file should be overwritten by the current operation.
        const OVERWRITE = 1 << 0;
    }
}

/// The File type of a file, maximum three ascii characters
///
/// # Variants
/// * [FileTransferType::Bin] - Binary files, generally programs
/// * [FileTransferType::Ini] - Ini files for program metadata and configuration
/// * [FileTransferType::Other] - Any other file type, including custom user types
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum FileTransferType {
    Bin,
    Ini,
    Other([u8; 3]),
}

impl FileTransferType {
    /// Converts the [FileTransferType] to a slice of 4 [u8]'s where the first three are the file's type and the last is a null terminator.
    ///
    /// # Example
    /// ```rust
    /// assert!(FileTransferType::Bin == *b"bin\0");
    /// ```
    pub fn to_bytes(self) -> [u8; 4] {
        match self {
            Self::Bin => *b"bin\0",
            Self::Ini => *b"ini\0",
            Self::Other(t) => [t[0], t[1], t[2], 0u8],
        }
    }

    /// Converts a slice of 4 [u8]'s into a [FileTransferType]
    pub fn from_bytes(v: &[u8; 4]) -> Self {
        match &v {
            [0x62, 0x69, 0x6e, 0x0] => Self::Bin,
            [0x69, 0x6e, 0x69, 0x0] => Self::Ini,
            _ => Self::Other([v[0], v[1], v[2]]),
        }
    }
}

/// The action to run when the transfer is complete.
///
/// # Variants
/// * [FileTransferComplete::DoNothing] - Does nothing when the file transfer is complete.
/// * [FileTransferComplete::RunProgram] - Runs the uploaded program when the transfer is complete.
/// * [FileTransferComplete::ShowRunScreen] - Shows the program run screen when the transfer is complete.
#[derive(Copy, Clone, Debug)]
pub enum FileTransferComplete {
    DoNothing = 0,
    RunProgram = 1,
    ShowRunScreen = 2,
    Halt = 3,
}

/// File metadata returned when requesting file metadata by index
#[derive(Copy, Clone, Debug)]
pub struct FileMetadataByIndex {
    /// The index of the file
    pub idx: u8,
    /// The type of the file
    pub file_type: FileTransferType,
    /// The length of the file
    pub length: u32,
    /// The address the file should be loaded at
    pub addr: u32,
    /// The crc32 of the file according to [crate::VEX_CRC32].
    pub crc: u32,
    /// The timestamp of when the file was last edited. I believe the unit is seconds since the year 2000
    pub timestamp: u32,
    /// The version of the file, pack such that 1.2.3.4 == 0x01020304
    pub version: u32,
    /// The name of the file
    pub name: [u8; 24],
}

/// File metadata returned when requesting file metadata by name
#[derive(Copy, Clone, Debug)]
pub struct FileMetadataByName {
    /// The VID of the linked file
    pub linked_vid: FileTransferVID,
    /// The type of the file
    pub file_type: FileTransferType,
    /// The length of the file
    pub length: u32,
    /// The address the file should be loaded at
    pub addr: u32,
    /// The crc32 of the file according to [crate::VEX_CRC32].
    pub crc: u32,
    /// The timestamp of when the file was last edited. I believe the unit is seconds since the year 2000
    pub timestamp: u32,
    /// The version of the file, pack such that 1.2.3.4 == 0x01020304
    pub version: u32,
    /// The filename of the linked file
    pub linked_filename: [u8; 24],
}
