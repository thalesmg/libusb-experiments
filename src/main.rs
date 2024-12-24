use std::{ptr::NonNull, time::Duration};

use rusb::{
    constants::{LIBUSB_CLASS_WIRELESS, LIBUSB_TRANSFER_FREE_TRANSFER}, ffi::{libusb_alloc_transfer, libusb_attach_kernel_driver, libusb_claim_interface, libusb_detach_kernel_driver, libusb_fill_bulk_transfer, libusb_fill_control_transfer, libusb_fill_interrupt_transfer, libusb_handle_events, libusb_set_auto_detach_kernel_driver, libusb_submit_transfer, libusb_transfer, libusb_transfer_cb_fn}, request_type, Direction, GlobalContext, Recipient, RequestType, UsbContext
};

enum HCIEvent {
    InquiryResultWithRSSI{
        bdaddr: [u8; 6]
    },
}

fn parse_hci_event(data: &[u8]) -> Option<HCIEvent> {
    match data[0] {
        0x22 => {
            let length = data[1];
            let number_of_responses = data[2];
            let mut bdaddr: [u8; 6] = data[3..9].try_into().unwrap();
            bdaddr.reverse();
            Some(HCIEvent::InquiryResultWithRSSI { bdaddr })
        },
        _ => {
            eprintln!("unknown packet type: {}", data[0]);
            None
        }
    }
}

// https://github.com/a1ien/rusb/issues/62#issuecomment-1148011618
extern "system" fn transfer_finished(transfer_ptr: *mut libusb_transfer) {
    println!("\n\naaaaaaaaaaaaaaaaaaaaaaaaaa\n\n");
    unsafe {
        let transfer: &mut libusb_transfer = &mut *transfer_ptr;
        dbg!(transfer.length);
        dbg!(transfer.actual_length);
        dbg!(transfer.endpoint);
        dbg!(transfer.status);
        dbg!(transfer.num_iso_packets);
        dbg!(transfer.buffer);
        dbg!(*transfer.buffer);
        let slice = std::slice::from_raw_parts(transfer.buffer, transfer.actual_length as usize);
        println!("{:#04x?}", slice);
        if let Some(HCIEvent::InquiryResultWithRSSI { bdaddr }) = parse_hci_event(slice) {
            println!("bdaddr: {:#04x?}", bdaddr);
        }
        let user_data = transfer.user_data;
        if !user_data.is_null() {
            dbg!(user_data);
        }
    }
}

fn main() {
    println!("Hello, world!");
    const SUBCLASS: u8 = 0x01;
    const PROTOCOL_BLUETOOTH: u8 = 0x01;
    let mut bts = vec![];
    for device in rusb::devices().unwrap().iter() {
        dbg!(&device);
        dbg!(device.address());
        dbg!(device.bus_number());
        let descriptor = device.device_descriptor().unwrap();
        dbg!(&descriptor);
        dbg!(descriptor.protocol_code());
        dbg!(descriptor.sub_class_code());
        dbg!(descriptor.class_code());

        if descriptor.class_code() == LIBUSB_CLASS_WIRELESS
            && descriptor.sub_class_code() == SUBCLASS
            && descriptor.protocol_code() == PROTOCOL_BLUETOOTH
        {
            println!("Found BT!!! {:?}", device);

            let cfg_descriptor = device.active_config_descriptor().unwrap();
            for interface in cfg_descriptor.interfaces() {
                for if_descriptor in interface.descriptors() {
                    for ep_descriptor in if_descriptor.endpoint_descriptors() {
                        dbg!(ep_descriptor);
                    }
                }
            }

            bts.push(device);
        }
    }
    dbg!(&bts);
    bts.sort_by(|a, b| a.address().cmp(&b.address()));
    dbg!(&bts);
    let device = bts[0].clone();
    let dev_handle = device.open().unwrap();

    let dev_handle_raw = unsafe { NonNull::new_unchecked(dev_handle.as_raw()) };
    let enable = 1;
    if dbg!(unsafe{libusb_set_auto_detach_kernel_driver(dev_handle_raw.as_ptr(), enable)}) < 0 {
        eprintln!("failed to set auto detach kernel driver");
        return
    }
    let interface_number = 0;
    // if dbg!(unsafe{libusb_detach_kernel_driver(dev_handle_raw.as_ptr(), interface_number)}) < 0 {
    //     eprintln!("failed to detach kernel driver");
    //     return
    // }
    if dbg!(unsafe{libusb_claim_interface(dev_handle_raw.as_ptr(), interface_number)}) < 0 {
        // eprintln!(
        //     "failed to claim interface: {}",
        //     Errno::last().desc()
        // );
        eprintln!("failed to claim interface");
        return
    }

    // println!("resetting.....");
    // dev_handle.reset().unwrap();

    // println!("submitting transfer....");
    // let dev_handle_raw = unsafe { NonNull::new_unchecked(dev_handle.as_raw()) };
    // // let transfer = unsafe { NonNull::new_unchecked(libusb_alloc_transfer(0)) };
    // let transfer = unsafe { libusb_alloc_transfer(0) };
    // let length = 600;
    // let mut buffer = vec![0; length];
    // unsafe {
    //     let transfer_ptr: &mut libusb_transfer = &mut *transfer;
    //     transfer_ptr.flags |= LIBUSB_TRANSFER_FREE_TRANSFER;
    //     let timeout = 5_000;
    //     let callback = transfer_finished as libusb_transfer_cb_fn;
    //     libusb_fill_control_transfer(
    //         transfer,
    //         dev_handle_raw.as_ptr(),
    //         buffer.as_mut_ptr(),
    //         callback,
    //         std::ptr::null_mut(),
    //         timeout
    //     );
    //     libusb_submit_transfer(transfer);
    // }
    // println!("submitted transfer");
    // std::thread::sleep(Duration::from_millis(5_000));


    // works!
    let request_type = 0x20;
    let value = 0x0000;
    let request = 0x00;
    let index = 0x00; //????
    let timeout = Duration::from_millis(4_000);
    let buf = [0x01, 0x04, 0x05, 0x00, 0x8b, 0x9e, 0x03, 0x00];
    println!("writing.....");
    dbg!(dev_handle.write_control(request_type, request, value, index, &buf, timeout)).unwrap();
    println!("wrote");
    println!("{:#04x?}", buf);

    // std::thread::sleep(Duration::from_millis(2_000));

    println!("submitting transfer....");
    let dev_handle_raw = unsafe { NonNull::new_unchecked(dev_handle.as_raw()) };
    // let transfer = unsafe { NonNull::new_unchecked(libusb_alloc_transfer(0)) };
    let transfer = unsafe { libusb_alloc_transfer(0) };
    let length = 600;
    let mut buffer = vec![0; length];
    unsafe {
        let transfer_ptr: &mut libusb_transfer = &mut *transfer;
        transfer_ptr.flags |= LIBUSB_TRANSFER_FREE_TRANSFER;
        let timeout = 5_000;
        let endpoint = 0x81;
        let callback = transfer_finished as libusb_transfer_cb_fn;
        libusb_fill_interrupt_transfer(
            transfer,
            dev_handle_raw.as_ptr(),
            endpoint,
            buffer.as_mut_ptr(),
            length as i32,
            callback,
            std::ptr::null_mut(),
            timeout
        );
        libusb_submit_transfer(transfer);
    }
    println!("submitted transfer");

    println!("submitting transfer....");
    let dev_handle_raw = unsafe { NonNull::new_unchecked(dev_handle.as_raw()) };
    // let transfer = unsafe { NonNull::new_unchecked(libusb_alloc_transfer(0)) };
    let transfer = unsafe { libusb_alloc_transfer(0) };
    let length = 600;
    let mut buffer = vec![0; length];
    unsafe {
        let transfer_ptr: &mut libusb_transfer = &mut *transfer;
        transfer_ptr.flags |= LIBUSB_TRANSFER_FREE_TRANSFER;
        let timeout = 5_000;
        let endpoint = 0x81;
        let callback = transfer_finished as libusb_transfer_cb_fn;
        libusb_fill_interrupt_transfer(
            transfer,
            dev_handle_raw.as_ptr(),
            endpoint,
            buffer.as_mut_ptr(),
            length as i32,
            callback,
            std::ptr::null_mut(),
            timeout
        );
        libusb_submit_transfer(transfer);
    }
    println!("submitted transfer");



    device.context().handle_events(Some(Duration::from_millis(2_000))).unwrap();

    // let mut buf = Vec::with_capacity(600);
    // let endpoint = 0x81;
    // println!("reading interrupt.....");
    // dev_handle.read_interrupt(endpoint, &mut buf, timeout).unwrap();
    // println!("{:#04x?}", buf);



    // let mut buf = vec![0; 600];
    // let endpoint = 0x82;
    // println!("reading bulk.....");
    // dbg!(dev_handle.read_bulk(endpoint, &mut buf, timeout)).unwrap();
    // println!("{:#04x?}", buf);


    // println!("submitting transfer....");
    // let dev_handle_raw = unsafe { NonNull::new_unchecked(dev_handle.as_raw()) };
    // // let transfer = unsafe { NonNull::new_unchecked(libusb_alloc_transfer(0)) };
    // let transfer = unsafe { libusb_alloc_transfer(0) };
    // let length = 600;
    // let mut buffer = vec![0; length];
    // unsafe {
    //     let transfer_ptr: &mut libusb_transfer = &mut *transfer;
    //     transfer_ptr.flags |= LIBUSB_TRANSFER_FREE_TRANSFER;
    //     let timeout = 5_000;
    //     let endpoint = 0x81;
    //     let callback = transfer_finished as libusb_transfer_cb_fn;
    //     libusb_fill_bulk_transfer(
    //         // transfer.as_ptr(),
    //         transfer,
    //         dev_handle_raw.as_ptr(),
    //         endpoint,
    //         buffer.as_mut_ptr(),
    //         length as i32,
    //         callback,
    //         std::ptr::null_mut(),
    //         timeout,
    //     );
    //     libusb_submit_transfer(transfer);
    // }
    // println!("submitted transfer");


    // std::thread::sleep(Duration::from_millis(1_000));

    // println!("reading control.....");
    // let request_type = rusb::request_type(Direction::In, RequestType::Standard, Recipient::Device);
    // let mut buf = Vec::with_capacity(600);
    // dev_handle.read_control(request_type, request, value, index, &mut buf, timeout).unwrap();
    // println!("{:#04x?}", buf);

    // let mut buf = Vec::with_capacity(600);
    // let endpoint = 0x81;
    // println!("reading interrupt.....");
    // dev_handle.read_interrupt(endpoint, &mut buf, timeout).unwrap();
    // println!("{:#04x?}", buf);

    // Println!("reading.....");
    // let request_type = rusb::request_type(Direction::In, RequestType::Standard, Recipient::Device);
    // dev_handle.read_control(request_type, request, value, index, &mut buf, timeout).unwrap();
    // println!("{:#04x?}", buf);

    // let mut buf = Vec::with_capacity(255);
    // let endpoint = 0x81;
    // println!("reading bulk.....");
    // dbg!(dev_handle.read_bulk(endpoint, &mut buf, timeout)).unwrap();
    // println!("{:#04x?}", buf);



    // println!("submitting transfer....");
    // let dev_handle_raw = unsafe { NonNull::new_unchecked(dev_handle.as_raw()) };
    // // let transfer = unsafe { NonNull::new_unchecked(libusb_alloc_transfer(0)) };
    // let transfer = unsafe { libusb_alloc_transfer(0) };
    // let length = 600;
    // let mut buffer = vec![0; length];
    // unsafe {
    //     let transfer_ptr: &mut libusb_transfer = &mut *transfer;
    //     transfer_ptr.flags |= LIBUSB_TRANSFER_FREE_TRANSFER;
    //     let timeout = 5_000;
    //     let endpoint = 0x81;
    //     let callback = transfer_finished as libusb_transfer_cb_fn;
    //     libusb_fill_bulk_transfer(
    //         // transfer.as_ptr(),
    //         transfer,
    //         dev_handle_raw.as_ptr(),
    //         endpoint,
    //         buffer.as_mut_ptr(),
    //         length as i32,
    //         callback,
    //         std::ptr::null_mut(),
    //         timeout,
    //     );
    //     libusb_submit_transfer(transfer);
    // }
    // println!("submitted transfer");
    // std::thread::sleep(Duration::from_millis(5_000));
    // println!("{:#04x?}", buffer);




    // println!("submitting inquiry as transfer....");
    // let dev_handle_raw = unsafe { NonNull::new_unchecked(dev_handle.as_raw()) };
    // // let transfer = unsafe { NonNull::new_unchecked(libusb_alloc_transfer(0)) };
    // let transfer = unsafe { libusb_alloc_transfer(0) };
    // // let length = 600;
    // let mut buffer = vec![0x01, 0x04, 0x05, 0x00, 0x8b, 0x9e, 0x03, 0x00];
    // unsafe {
    //     let transfer_ptr: &mut libusb_transfer = &mut *transfer;
    //     transfer_ptr.flags |= LIBUSB_TRANSFER_FREE_TRANSFER;
    //     let timeout = 3_000;
    //     let callback = transfer_finished as libusb_transfer_cb_fn;
    //     libusb_fill_control_transfer(
    //         transfer,
    //         dev_handle_raw.as_ptr(),
    //         buffer.as_mut_ptr(),
    //         callback,
    //         std::ptr::null_mut(),
    //         timeout,
    //     );
    //     libusb_submit_transfer(transfer);
    // }
    // println!("submitted transfer");
    // std::thread::sleep(Duration::from_millis(5_000));
    // println!("buffer after {:#04x?}", buffer);



    // let mut buf = Vec::with_capacity(599);
    // let endpoint = 0x81;
    // println!("reading interrupt.....");
    // dev_handle.read_interrupt(endpoint, &mut buf, timeout).unwrap();
    // println!("{:#04x?}", buf);

    std::thread::sleep(Duration::from_millis(6_000));

    dbg!(unsafe{libusb_attach_kernel_driver(dev_handle_raw.as_ptr(), interface_number)});

    println!("bye");
}
