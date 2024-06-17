use vexv5_serial::{
    commands::file::{UploadFile, COLD_START},
    protocol::{Program, ProgramIniConfig, Project},
    v5::{FileTransferComplete, FileTransferType, FileTransferVID},
};

#[tokio::main]
async fn main() {
    // Find all vex devices on the serial ports
    let vex_ports = vexv5_serial::devices::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();

    let ini = ProgramIniConfig {
        program: Program {
            description: "made with vexide".to_string(),
            icon: "default.bmp".to_string(),
            iconalt: String::new(),
            slot: 2,
            name: "vexide".to_string(),
        },
        project: Project {
            ide: "vexide".to_string(),
        },
    };
    println!("{}", serde_ini::to_string(&ini).unwrap());
    let ini = serde_ini::to_vec(&ini).unwrap();
    
    let file_transfer = UploadFile {
        filename: "happy.ini".to_string(),
        filetype: FileTransferType::Ini,
        vendor: None,
        data: ini,
        target: None,
        load_addr: COLD_START,
        linked_file: None,
        after_upload: FileTransferComplete::ShowRunScreen,
    };
    device.execute_command(file_transfer).await.unwrap();

    let file_bytes = include_bytes!("./basic.bin");

    let file_transfer = UploadFile {
        filename: "happy.bin".to_string(),
        filetype: FileTransferType::Bin,
        vendor: None,
        data: file_bytes.to_vec(),
        target: None,
        load_addr: COLD_START,
        linked_file: None,
        after_upload: FileTransferComplete::ShowRunScreen,
    };
    device.execute_command(file_transfer).await.unwrap();
}
