use vexv5_serial::{
    commands::file::{ProgramData, UploadProgram},
    packets::file::FileExitAtion,
};

#[tokio::main]
async fn main() {
    // Find all vex devices on the serial ports
    let vex_ports = vexv5_serial::devices::genericv5::find_generic_devices().unwrap();

    // Open the device
    let mut device = vex_ports[0].open().unwrap();
    let cold_bytes = include_bytes!("./basic.bin").to_vec();
    device
        .execute_command(UploadProgram {
            name: "quick".to_string(),
            description: "A basic vexide program".to_string(),
            icon: "USER029x.bmp".to_string(),
            program_type: "vexide".to_string(),
            slot: 1,
            data: ProgramData::Cold(cold_bytes),
            after_upload: FileExitAtion::RunProgram,
        })
        .await
        .unwrap();
}
