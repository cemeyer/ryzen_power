use nix::ioctl_readwrite;
use std::{fs::File, os::unix::prelude::*};
use sysctl::{self, CtlValue, Sysctl};

// Thoroughly inspired by https://github.com/ryanriske/ZenStates-Linux/blob/master/zenstates.py

#[repr(C)]
pub struct cpuctl_msr_args {
    msr: std::os::raw::c_uint,
    data: u64,
}

ioctl_readwrite!(read_msr, b'c', 1, cpuctl_msr_args);
//ioctl_readwrite!(write_msr, b'c', 2, cpuctl_msr_args);

fn open_cpu(i: usize) -> RawFd {
    let path = format!("/dev/cpuctl{}", i);
    let f = File::open(path).unwrap();
    f.into_raw_fd()
}

fn dump_stats(cores: usize, threads_per_core: usize) {
    let mut msr_values = vec![0; cores];
    let mut fds: Vec<RawFd> = vec![0; cores];

    let mut i = 0;
    for core in 0..cores {
        fds[core] = open_cpu(i);
        i += threads_per_core;
    }

    for core in 0..cores {
        let mut msr_one = cpuctl_msr_args {
            msr: 0xC0010293,
            data: 0,
        };
        let ptr = &mut msr_one;
        unsafe { read_msr(fds[core], ptr as *mut _) }.unwrap();

        msr_values[core] = msr_one.data;
    }

    for core in 0..cores {
        let val = msr_values[core];

        let vid = (val >> 14) & 0xff;
        let did = (val >> 8) & 0x3f;
        let fid = val & 0xff;

        let ratio = 25.0 * (fid as f64) / (12.5 * (did as f64));
        let freq_ghz = ratio / 10.0;

        let volts = 1.55 - (vid as f64) * 0.00625;

        println!("{:<04.2} GHz @ {:<04.2}V", freq_ghz, volts);
    }
}

fn main() {
    let cores = sysctl::Ctl::new("kern.smp.cores").unwrap().value().unwrap();
    let threads_per_core = sysctl::Ctl::new("kern.smp.threads_per_core")
        .unwrap()
        .value()
        .unwrap();

    let cores = match cores {
        CtlValue::Int(x) => x,
        _ => panic!(),
    };
    let threads_per_core = match threads_per_core {
        CtlValue::Int(x) => x,
        _ => panic!(),
    };

    dump_stats(cores as usize, threads_per_core as usize);

    let cpu_sysctl = "dev.amdtemp.0.core0.sensor0";
    let cputemp = match sysctl::Ctl::new(cpu_sysctl).unwrap().value().unwrap() {
        CtlValue::Temperature(temp) => temp.celsius(),
        _ => panic!(),
    };
    println!("{}: {:.1}C", cpu_sysctl, cputemp);
    let dimm_sysctl = "dev.jedec_dimm.0.temp";
    let dimmtemp = match sysctl::Ctl::new(dimm_sysctl).unwrap().value().unwrap() {
        CtlValue::Temperature(temp) => temp.celsius(),
        _ => panic!(),
    };
    println!("{}: {:.1}C", dimm_sysctl, dimmtemp);
}
