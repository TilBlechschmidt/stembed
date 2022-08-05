use std::time::{Duration, Instant};

use cofit::{Transport, UsbHidTransport};
use hidapi::HidApi;
use runtime::api::RuntimeAPI;
use tokio::select;

const USAGE_PAGE_VENDOR: u16 = 0xFF00;
const USAGE_EMBEDDED_STENO: u16 = 0x42;
const DEVICE_VID: u16 = 0xC0DE;
const DEVICE_PID: u16 = 0xCAFE;

const DICT_OFFSET: u32 = 0; // 4096 * 700;

#[tokio::main]
async fn main() {
    let api = HidApi::new().expect("failed to setup HID API");

    let device = api
        .device_list()
        .filter(|d| d.vendor_id() == DEVICE_VID && d.product_id() == DEVICE_PID)
        .filter(|d| d.usage_page() == USAGE_PAGE_VENDOR && d.usage() == USAGE_EMBEDDED_STENO)
        .map(|d| d.open_device(&api))
        .next()
        .expect("no device found")
        .expect("failed to open device");

    let transport = UsbHidTransport::new(device);

    let (api_task, api) = RuntimeAPI::new(&transport);

    let main_task = async move {
        api.reset().await;

        write_test(&api).await;
        // for _ in 0..1 {
        //     println!("-------");
        //     verify_test(&api).await;
        //     tokio::time::sleep(Duration::from_millis(2000)).await;
        // }
    };

    select! {
        _ = api_task => {
            println!("api_task() completed")
        }
        _ = main_task => {
            println!("main_task() completed")
        }
    };
}

async fn write_test<'t, T: Transport<63>>(api: &RuntimeAPI<'t, T>) {
    let mut file =
        std::fs::read("/Users/tibl/Developer/Other/Steno/stembed/code/shittyengine/dict.bin")
            .expect("failed to read dict file");
    let mut flash = api.flash().await;

    while !(file.len() % 4 == 0) {
        file.push(255);
    }

    println!("flashing dictionary (len = {})", file.len());

    let mut write_task = flash.write(DICT_OFFSET, &file);
    while let Some(progress) = write_task.next().await.unwrap() {
        println!("{progress}");
    }
}

async fn verify_test<'t, T: Transport<63>>(api: &RuntimeAPI<'t, T>) {
    let file =
        std::fs::read("/Users/tibl/Developer/Other/Steno/stembed/code/shittyengine/dict.bin")
            .expect("failed to read dict file");
    let mut flash = api.flash().await;

    println!("verifying dictionary (len = {})", file.len());

    let chunk_size: u32 = 60 * 1024;
    for (i, chunk) in file.chunks(chunk_size as usize).enumerate() {
        let offset = DICT_OFFSET + i as u32 * chunk_size;
        let mut buf: Vec<u8> = (0..chunk.len()).map(|_| 0).collect();

        flash
            .read(offset, &mut buf)
            .await
            .expect("failed to read flash");

        if &buf != chunk {
            let delta = &buf
                .iter()
                .zip(chunk.iter())
                .fold(0, |acc, (a, b)| if a == b { acc } else { acc + 1 });
            eprintln!("chunk mismatch! ({delta} bytes)");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // let mut buf: Vec<u8> = (0..file.len()).map(|_| 0).collect();

    // flash
    //     .read(DICT_OFFSET, &mut buf)
    //     .await
    //     .expect("failed to read flash");

    // println!("comparing dict ...");
    // for i in 0..file.len() {
    //     if file[i] != buf[i] {
    //         eprintln!(
    //             "byte {} did not match (flash = {}, file = {}, remainder = {})",
    //             i,
    //             buf[i],
    //             file[i],
    //             i % 60
    //         );

    //         if (i != file.len() - 1 && file[i + 1] == buf[i]) || (i > 0 && file[i - 1] == buf[i]) {
    //             eprintln!("\tneighboring byte does match!");
    //         }

    //         if (i != file.len() - 1 && buf[i + 1] == buf[i]) || (i > 0 && buf[i - 1] == buf[i]) {
    //             eprintln!("\tneighboring byte is equal!");
    //         }
    //     }
    // }
}

async fn read_test<'t, T: Transport<63>>(api: &RuntimeAPI<'t, T>) {
    let mut flash = api.flash().await;
    let mut buf: Vec<u8> = (0..2usize.pow(16) * 48).map(|_| 0).collect();
    let offset = 0;

    let start = Instant::now();
    flash
        .read(offset, &mut buf)
        .await
        .expect("failed to read flash");

    dbg!(buf.len());
    dbg!(start.elapsed());
}
