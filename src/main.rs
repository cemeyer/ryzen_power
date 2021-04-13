use std::{fs::File, os::unix::prelude::*};

// Thoroughly inspired by https://github.com/ryanriske/ZenStates-Linux/blob/master/zenstates.py

#[cfg(target_os = "freebsd")]
mod platform {
    use super::*;
    use capsicum::{self, CapRights, Right};
    use nix::{ioctl_readwrite, request_code_readwrite};
    use std::mem::size_of;
    use sysctl::{self, CtlValue, Sysctl};

    #[repr(C)]
    struct cpuctl_msr_args {
        msr: std::os::raw::c_uint,
        data: u64,
    }

    const CPUCTL_RDMSR: std::os::raw::c_ulong =
        request_code_readwrite!(b'c', 1, size_of::<cpuctl_msr_args>());
    ioctl_readwrite!(raw_read_msr, b'c', 1, cpuctl_msr_args);
    //ioctl_readwrite!(raw_write_msr, b'c', 2, cpuctl_msr_args);

    pub(super) fn read_msr(core: RawFd, msr: u32) -> u64 {
        let mut msr_one = cpuctl_msr_args {
            msr,
            data: 0,
        };
        let ptr = &mut msr_one;
        unsafe { raw_read_msr(fds[core], ptr as *mut _) }.unwrap();
        msr_one.data
    }

    pub(super) fn open_cpu(i: usize) -> RawFd {
        let path = format!("/dev/cpuctl{}", i);
        let f = File::open(path).unwrap();
        f.into_raw_fd()
    }

    /// Enter Capsicum sandbox.  Restrict cpuctl fds to ioctl, and specifically rdmsr.
    pub(super) fn sandbox(fds: &Vec<RawFd>) {
        let rights = capsicum::RightsBuilder::new(Right::Ioctl)
            .finalize()
            .unwrap();
        let ioctls = capsicum::IoctlsBuilder::new(CPUCTL_RDMSR).finalize();

        for fd in fds.iter() {
            rights.limit(fd).unwrap();
            ioctls.limit(fd).unwrap();
        }

        capsicum::enter().unwrap();
    }

    pub(super) fn get_num_cores() -> usize {
        let cores = sysctl::Ctl::new("kern.smp.cores").unwrap().value().unwrap();
        match cores {
            CtlValue::Int(x) => x as usize,
            _ => panic!(),
        }
    }

    pub(super) fn get_num_threads_per_core() -> usize {
        let threads_per_core = sysctl::Ctl::new("kern.smp.threads_per_core")
            .unwrap()
            .value()
            .unwrap();
        match threads_per_core {
            CtlValue::Int(x) => x as usize,
            _ => panic!(),
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use num_cpus;

    pub(super) fn read_msr(core: RawFd, msr: u32) -> u64 {
        use nix::sys::uio::pread;
        use std::slice;

        let mut res: u64 = 0;
        let nb = pread(core,
                       unsafe {
                           slice::from_raw_parts_mut::<u8>(
                               &mut res as *mut u64 as *mut u8,
                               8)
                       },
                       msr as _)
            .unwrap();
        assert!(nb == 8);

        res
    }

    pub(super) fn open_cpu(i: usize) -> RawFd {
        let path = format!("/dev/cpu/{}/msr", i);
        let f = File::open(path).unwrap();
        f.into_raw_fd()
    }

    pub(super) fn get_num_cores() -> usize {
        num_cpus::get_physical()
    }

    pub(super) fn get_num_threads_per_core() -> usize {
        num_cpus::get() / num_cpus::get_physical()
    }

    pub(super) fn sandbox(_fds: &Vec<RawFd>) {
        // not implemented
    }
}

use platform::*;

fn open_cpus(cores: usize, threads_per_core: usize) -> Vec<RawFd> {
    let mut fds: Vec<RawFd> = vec![0; cores];

    let mut i = 0;
    for core in 0..cores {
        fds[core] = open_cpu(i);
        i += threads_per_core;
    }

    fds
}

fn dump_stats(fds: &Vec<RawFd>) {
    let cores = fds.len();
    let mut msr_values = vec![0; cores];

    for core in 0..cores {
        msr_values[core] = read_msr(fds[core], 0xC0010293);
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
    // Need to grab sysctls before sandboxing.
    let cores = get_num_cores();
    let threads_per_core = get_num_threads_per_core();

    #[cfg(target_os = "freebsd")]
    {
        let cpu_sysctl = "dev.amdtemp.0.core0.sensor0";
        let cputemp = match sysctl::Ctl::new(cpu_sysctl).unwrap().value().unwrap() {
            CtlValue::Temperature(temp) => temp.celsius(),
            _ => panic!(),
        };
        let dimm_sysctl = "dev.jedec_dimm.0.temp";
        let dimmtemp = match sysctl::Ctl::new(dimm_sysctl).unwrap().value().unwrap() {
            CtlValue::Temperature(temp) => temp.celsius(),
            _ => panic!(),
        };
    }

    // Preopen /dev/cpuctlN before sandboxing.
    let fds: Vec<RawFd> = open_cpus(cores, threads_per_core);

    //
    // RESTRICT FDS AND ENTER SANDBOX
    //
    sandbox(&fds);

    // Read MSRs, compute and display freq/power info.
    dump_stats(&fds);

    #[cfg(target_os = "freebsd")]
    {
        println!("{}: {:.1}C", cpu_sysctl, cputemp);
        println!("{}: {:.1}C", dimm_sysctl, dimmtemp);
    }
}
