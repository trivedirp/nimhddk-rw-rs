#![allow(non_snake_case)]
extern crate nimhddk_sys as ffi;
use log::debug;
use regex::Regex;
use std::{
    boxed::Box,
    ffi::c_void,
    fs::File,
    io::{self, BufRead},
    ops::FnMut,
    slice::from_raw_parts_mut,
};

unsafe extern "C" fn AO_handler(
    user_data: *const c_void,
    data: *mut f32,
    sample_index: usize,
    n_samples: usize,
) {
    let closure: &mut Box<dyn FnMut(&mut [f32], usize)> =
        &mut *(user_data as *mut Box<dyn FnMut(&mut [f32], usize)>);
    let slice = from_raw_parts_mut(data, n_samples);
    closure(slice, sample_index)
}
unsafe extern "C" fn AI_handler(
    user_data: *const c_void,
    data: *mut f32,
    sample_index: usize,
    n_samples: usize,
) {
    let closure: &mut Box<dyn FnMut(&mut [f32], usize)> =
        &mut *(user_data as *mut Box<dyn FnMut(&mut [f32], usize)>);
    let slice = from_raw_parts_mut(data, n_samples);
    closure(slice, sample_index)
}
unsafe extern "C" fn DO_handler(
    user_data: *const c_void,
    data: *mut std::os::raw::c_uchar,
    sample_index: usize,
    n_samples: usize,
) {
    let closure: &mut Box<dyn FnMut(&mut [bool], usize)> =
        &mut *(user_data as *mut std::boxed::Box<dyn std::ops::FnMut(&mut [bool], usize)>);
    let slice = from_raw_parts_mut(data as *mut bool, n_samples);
    closure(slice, sample_index)
}

#[derive(Clone, Copy)]
pub struct AnalogOutput(*mut ffi::tag_AO);
unsafe impl Sync for AnalogOutput {}
unsafe impl Send for AnalogOutput {}

#[derive(Clone, Copy)]
pub struct AnalogInput(*mut ffi::tag_AI);
unsafe impl Sync for AnalogInput {}
unsafe impl Send for AnalogInput {}

#[derive(Clone, Copy)]
pub struct DigitalOutput(*mut ffi::tag_DO);
unsafe impl Sync for DigitalOutput {}
unsafe impl Send for DigitalOutput {}

pub struct XSeriesDevice(*mut ffi::tag_device);

impl XSeriesDevice {
    pub fn new(i_bus: i32, i_device: i32) -> Option<Self> {
        unsafe {
            let device = ffi::create_device(i_bus, i_device);
            if device.as_mut().is_some() {
                Some(Self(device))
            } else {
                None
            }
        }
    }
    pub fn AO(&self) -> AnalogOutput {
        unsafe { AnalogOutput(ffi::AO(self.0)) }
    }
    pub fn AI(&self) -> AnalogInput {
        unsafe { AnalogInput(ffi::AI(self.0)) }
    }
    pub fn DO(&self) -> DigitalOutput {
        unsafe { DigitalOutput(ffi::DO(self.0)) }
    }
}

unsafe impl Sync for XSeriesDevice {}
unsafe impl Send for XSeriesDevice {}

impl Default for XSeriesDevice {
    fn default() -> Self {
        let file = File::open("/proc/nirlpk/lsdaq").expect("File /proc/nirlpk/lsdaq doesn't exist");
        let dev_info = io::BufReader::new(file)
            .lines()
            .last()
            .expect("/proc/nirlpk/lsdaq contains 0 lines")
            .expect("Failed to read line in /proc/nirlpk/lsdaq");
        let re = Regex::new(r"PXI([0-9]+)::([0-9]+)::INSTR").unwrap();
        let captures = re.captures(&dev_info).expect(&format!(
            "Failed to match \"{dev_info}\" with regular expression {}",
            re.as_str()
        ));
        let i_bus_str = captures.get(1).unwrap().as_str();
        let i_bus = i_bus_str
            .parse::<i32>()
            .expect("Unable to parse \"{i_bus_str}\" as i32");
        let i_device_str = captures.get(2).unwrap().as_str();
        let i_device = i_device_str
            .parse::<i32>()
            .expect("Unable to parse \"{i_device_str}\" as i32");
        debug!("Found NI device at {i_bus}, {i_device}");
        Self::new(i_bus, i_device).expect("Failed to create NI device")
    }
}

impl Drop for XSeriesDevice {
    fn drop(&mut self) {
        unsafe { ffi::destroy_device(self.0) }
    }
}

impl AnalogOutput {
    pub fn set(&self, channel: i32, value: f32) {
        unsafe { ffi::AO_set(self.0 as _, channel, value) }
    }
    pub fn sample_clock(&self) -> f64 {
        unsafe { ffi::AO_sample_clock(self.0 as _) }
    }
    pub fn samples_pending(&self) -> usize {
        unsafe { ffi::AO_samples_pending(self.0 as _) }
    }
    pub fn add_streaming_channel<F>(&self, channel: i32, callback: F)
    where
        F: FnMut(&mut [f32], usize),
        F: 'static,
    {
        let cb: Box<Box<dyn FnMut(&mut [f32], usize)>> = Box::new(Box::new(callback));
        unsafe {
            ffi::AO_add_streaming_channel(
                self.0 as _,
                channel,
                Some(AO_handler),
                Box::into_raw(cb) as *mut _,
            );
        }
    }
    pub fn start(&self) {
        self.start_as_follower(false);
    }
    pub fn start_as_follower(&self, follower: bool) {
        unsafe { ffi::AO_start(self.0 as _, follower as u8) }
    }
    pub fn stop(&self) {
        unsafe { ffi::AO_stop(self.0 as _) }
    }
}

impl AnalogInput {
    pub fn sample_clock(&self) -> f64 {
        unsafe { ffi::AI_sample_clock(self.0 as _) }
    }
    pub fn samples_available(&self) -> usize {
        unsafe { ffi::AI_samples_available(self.0 as _) }
    }
    pub fn add_streaming_channel(&self, channel: i32) {
        unsafe {
            ffi::AI_add_streaming_channel(self.0 as _, channel);
        }
    }
    pub fn set_ondemand(&self, channel: i32) {
        unsafe { ffi::AI_set_ondemand(self.0 as _, channel) }
    }
    pub fn stop_ondemand(&self, channel: i32) {
        unsafe { ffi::AI_stop_ondemand(self.0 as _, channel) }
    }
    pub fn read_ondemand(&self, channel: i32)-> f32 {
        unsafe { ffi::AI_read_ondemand(self.0 as _, channel) }
    }
    /* pub fn datavec_read(&self, data_read_vec:Vec<f64> ) {
        unsafe { ffi::AI_data_read(self.0 as _, data_read_vec) }
    } */
    pub fn start_stream(&self) {
        self.start_as_follower(false);
    }
    pub fn start_as_follower(&self, follower: bool) {
        unsafe { ffi::AI_start_stream(self.0 as _, follower as u8) }
    }
    pub fn stop_stream(&self) {
        unsafe { ffi::AI_stop_stream(self.0 as _) }
    }
}

impl DigitalOutput {
    pub fn set(&self, value: u32) {
        unsafe { ffi::DO_set(self.0 as _, value) }
    }
    pub fn sample_clock(&self) -> f64 {
        unsafe { ffi::DO_sample_clock(self.0 as _) }
    }
    pub fn samples_pending(&self) -> usize {
        unsafe { ffi::DO_samples_pending(self.0 as _) }
    }
    pub fn add_streaming_line<F>(&self, channel: i32, callback: F)
    where
        F: FnMut(&mut [bool], usize),
        F: 'static,
    {
        let cb: Box<Box<dyn FnMut(&mut [bool], usize)>> = Box::new(Box::new(callback));
        unsafe {
            ffi::DO_add_streaming_line(
                self.0 as _,
                channel,
                Some(DO_handler),
                Box::into_raw(cb) as *mut _,
            );
        }
    }
    pub fn start(&self) {
        self.start_as_follower(false);
    }
    pub fn start_as_follower(&self, follower: bool) {
        unsafe { ffi::DO_start(self.0 as _, follower as u8) }
    }
    pub fn stop(&self) {
        unsafe { ffi::DO_stop(self.0 as _) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{Arc, Mutex},
        thread::{self, sleep},
        time::{Duration, Instant},
    };
    fn do_clk(t_s: f64) -> usize {
        let do_sample_clock = 10e6;
        (t_s * do_sample_clock).round() as usize
    }
    fn ao_clk(t_s: f64) -> usize {
        let ao_sample_clock = 1e6;
        (t_s * ao_sample_clock).round() as usize
    }
    fn ai_clk(t_s: f64) -> usize {
        let ai_sample_clock = 1e6;
        (t_s * ai_sample_clock).round() as usize
    }
    pub struct IrCamera {
        pub period_s: f64,
        pub half_period_s: f64,
    }
    impl IrCamera {
        pub fn new() -> IrCamera {
            let period_s = 4e-3;
            let half_period_s = period_s / 2.0;
            IrCamera {
                period_s,
                half_period_s,
            }
        }
    }
    pub struct Waveforms {
        pub ir_camera: IrCamera,
    }
    impl Waveforms {
        pub fn new() -> Waveforms {
            let ir_camera = IrCamera::new();
            Waveforms { ir_camera }
        }
        fn AO_get_data0(&self, data: &mut [f32], start: usize) {
            let period_sclk = ao_clk(self.ir_camera.period_s);
            for (i, d) in data.iter_mut().enumerate() {
                *d = ((start + i) % period_sclk) as f32 / period_sclk as f32;
            }
        }
        fn AI_get_data0(&self, data: &mut [f32], start: usize) {
            let period_sclk = ai_clk(self.ir_camera.period_s);
            for (i, d) in data.iter_mut().enumerate() {
                *d = 0.0f32;
            }
        }
        fn DO_get_data0(&self, data: &mut [bool], start: usize) {
            let period_sclk = do_clk(self.ir_camera.period_s);
            let half_period_sclk = do_clk(self.ir_camera.half_period_s);
            for (i, d) in data.iter_mut().enumerate() {
                *d = (start + i) % period_sclk < half_period_sclk;
            }
        }
    }
    #[test]
    fn device() {
        let _daq = XSeriesDevice::new(11, 0).unwrap();
    }
    #[test]
    fn DO_on_demand() {
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let DO = daq.DO();
        DO.set(0b0101);
        sleep(Duration::from_secs(1));
        DO.set(0b1010);
        sleep(Duration::from_secs(1));
        DO.set(0b0000);
    }
    #[test]
    fn AO_on_demand() {
        let channel = 0;
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let AO = daq.AO();
        AO.set(channel, 0f32);
        for i in 1..=10 {
            sleep(Duration::from_millis(100));
            AO.set(channel, i as f32 / 10f32);
        }
        for i in (0..10).rev() {
            sleep(Duration::from_millis(100));
            AO.set(channel, i as f32 / 10f32);
        }
    }
    #[test]
    fn AI_on_demand() {
        let channel = 0;
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let AI = daq.AI();
        let mut val:f32 = -1111.0;
        AI.set_ondemand(channel);
        val= AI.read_ondemand(channel);
        AI.stop_ondemand(channel);
        println!("data_read val:  {} \t", val);
        /* for i in 1..=3 {
            sleep(Duration::from_millis(100));
            val = AI.read_ondemand(channel);
            println!("data_read val:  {} \t", val);
        } */
    }
    #[test]
    fn DO_streaming() {
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let DO = daq.DO();
        let waveforms = Arc::new(Mutex::new(Waveforms::new()));
        let waveforms0 = Arc::clone(&waveforms);
        let waveforms1 = Arc::clone(&waveforms);
        DO.add_streaming_line(0, move |data, i| {
            waveforms0.lock().unwrap().DO_get_data0(data, i)
        });
        DO.add_streaming_line(1, move |data, i| {
            waveforms1.lock().unwrap().DO_get_data0(data, i)
        });
        DO.start();
        sleep(Duration::from_secs(5));
        DO.stop();
    }
    #[test]
    fn DO_streaming_long() {
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let DO = daq.DO();
        let waveforms = Arc::new(Mutex::new(Waveforms::new()));
        let waveforms0 = Arc::clone(&waveforms);
        let waveforms1 = Arc::clone(&waveforms);
        DO.add_streaming_line(0, move |data, i| {
            waveforms0.lock().unwrap().DO_get_data0(data, i)
        });
        DO.add_streaming_line(1, move |data, i| {
            waveforms1.lock().unwrap().DO_get_data0(data, i)
        });
        DO.start();
        sleep(Duration::from_secs(600));
        DO.stop();
    }
    #[test]
    fn AO_streaming() {
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let AO = daq.AO();
        let waveforms = Waveforms::new();
        AO.add_streaming_channel(0, move |data, i| waveforms.AO_get_data0(data, i));
        AO.start();
        thread::sleep(Duration::from_secs(5));
        AO.stop();
    }
    #[test]
    fn AI_streaming() {
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let AI = daq.AI();
        let waveforms = Waveforms::new();
        AI.add_streaming_channel(0);
        AI.start_stream();
        thread::sleep(Duration::from_secs(1));
        AI.stop_stream();
        // let data_read_vec:Vec<f64>
        // AI.data_read(data_read_vec);
        println!("data_read vector: \t");
        //  println!("{}", data_read_vec[1]);
    }
    #[test]
    fn AO_streaming_on_demand() {
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let AO = daq.AO();
        let waveforms = Arc::new(Waveforms::new());
        let waveforms2 = waveforms.clone();
        AO.add_streaming_channel(1, move |data, i| waveforms.AO_get_data0(data, i));
        AO.add_streaming_channel(3, move |data, i| waveforms2.AO_get_data0(data, i));
        AO.set(0, 0.0);
        AO.set(2, 0.0);
        AO.start();
        for i in 1..=10 {
            sleep(Duration::from_millis(100));
            AO.set(0, i as f32 / 10f32);
            AO.set(2, i as f32 / 10f32);
        }
        for i in (0..10).rev() {
            sleep(Duration::from_millis(100));
            AO.set(0, i as f32 / 10f32);
            AO.set(2, i as f32 / 10f32);
        }
        AO.stop();
    }
    
    #[test]
    fn AO_realtime_streaming() {
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let AO = daq.AO();
        let DO = daq.DO();
        DO.set(0);
        AO.add_streaming_channel(0, move |data, start| {
            let sweep_sclk = 500000;
            let margin_sclk = 100000;
            if start % sweep_sclk == 0 {
                while AO.samples_pending() > margin_sclk {}
                DO.set(1);
            }
            for (i, d) in data.iter_mut().enumerate() {
                *d = ((start + i) % sweep_sclk) as f32 / sweep_sclk as f32;
            }
        });
        AO.start();
        let now = Instant::now();
        while now.elapsed() < Duration::from_secs(5) {
            println!("{}", AO.samples_pending());
            thread::sleep(Duration::from_millis(10));
        }
        AO.stop();
        DO.set(0);
    }
    #[test]
    fn multithreading() {
        let daq = XSeriesDevice::new(11, 0).unwrap();
        let thread_join = thread::spawn(move || {
            daq.DO().set(0);
        });
        thread_join.join().unwrap();
    }
    #[test]
    fn camera_strober() {
        let daq = XSeriesDevice::default();
        let DO = daq.DO();
        DO.add_streaming_line(5, move |data, start| {
            for (i, d) in data.iter_mut().enumerate() {
                *d = (start + i) % 40000 < 20000; // square wave on port0/line5 at 250 Hz
            }
        });
        DO.start();
        println!("Generating digital waveform, press enter to terminate");
        let mut buf = String::new();
        io::stdin().read_line(&mut buf).unwrap();
        DO.stop();
    }
}
