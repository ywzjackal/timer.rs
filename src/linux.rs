#[link(name = "rt")]
// use std::sync::mpsc::{channel, Sender, Receiver};
use event::Holder;
use std::time::Duration;
use std::io::{Result, Error};
use std::ops::Drop;
use libc::{c_int, c_long, pthread_t, pthread_self, c_void};

const SIGEV_SIGNAL: i32 = 0;
const SIGEV_NONE: i32 = 1;
const SIGEV_THREAD: i32 = 2;
const SIGEV_THREAD_ID: i32 = 4;

const SCHED_OTHER: i32 = 0;
const SCHED_FIFO: i32 = 1;
const SCHED_RR: i32 = 2;

pub const CLOCK_REALTIME: i32 = 0;
// Monotonic system-wide clock.
pub const CLOCK_MONOTONIC: i32 = 1;
// High-resolution timer from the CPU.
const CLOCK_PROCESS_CPUTIME_ID: i32 = 2;
// Thread-specific CPU-time clock.
const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
// Monotonic system-wide clock, not adjusted for frequency scaling.
const CLOCK_MONOTONIC_RAW: i32 = 4;
// Identifier for system-wide realtime clock, updated only on ticks.
const CLOCK_REALTIME_COARSE: i32 = 5;
// Monotonic system-wide clock, updated only on ticks.
const CLOCK_MONOTONIC_COARSE: i32 = 6;
// Monotonic system-wide clock that includes time spent in suspension.
const CLOCK_BOOTTIME: i32 = 7;
// Like CLOCK_REALTIME but also wakes suspended system.
const CLOCK_REALTIME_ALARM: i32 = 8;
// Like CLOCK_BOOTTIME but also wakes suspended system.
const CLOCK_BOOTTIME_ALARM: i32 = 9;
// Like CLOCK_REALTIME but in International Atomic Time.
const CLOCK_TAI: i32 = 11;
// Flag to indicate time is absolute.
const TIMER_ABSTIME: i32 = 1;

type TimerId = c_long;

type CallbackFunctionParam = *mut c_void;

fn get_thread_id() -> pthread_t {
    unsafe { pthread_self() }
}

#[repr(C)]
pub struct Timer {
    timer_id: TimerId,
    pub on_arrived: Holder<i32>,
}

extern "C" fn cb(timer: CallbackFunctionParam) {
    let timer = timer as *mut Timer;
    if timer.is_null() {
        panic!("invalid cb param in timer callback fn!!");
    }
    let mut overrun = unsafe { timer_getoverrun((*timer).timer_id) };
    unsafe { (*timer).on_arrived.invoke(&mut overrun) };
}

impl Timer {
    // Create an empty Timer Struct
    pub fn new() -> Timer {
        Timer {
            timer_id: 0,
            on_arrived: Holder::new(),
        }
    }

    // Setup timer in ticker mode.
    pub fn ticker(&mut self, clock_type: i32, policy: i32) -> Result<()> {
        let mut pthread_attr = pthread_attr_t::new();
        let timer_id_ptr: *mut TimerId = &mut self.timer_id;
        let mut sigevent = try!(sigevent_t::with_callback(cb,
                                                          self as *mut Timer as *mut c_void,
                                                          policy,
                                                          &mut pthread_attr));
        if unsafe { timer_create(clock_type, &mut sigevent, timer_id_ptr) } != 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    // Start ticker with TIMER_ABSTIME mode.
    pub fn start_abstime(&self, interval: Duration, from: Duration) -> Result<()> {
        let interval = timespec_t::new(interval.as_secs() as c_long,
                                       interval.subsec_nanos() as c_long);
        let start = timespec_t::new(from.as_secs() as c_long, from.subsec_nanos() as c_long);
        let mut itime = itimerspec_t::with_value(interval, start);
        let rt = unsafe {
            timer_settime(self.timer_id,
                          TIMER_ABSTIME,
                          &mut itime,
                          0 as *mut itimerspec_t)
        };
        if rt != 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    // Start ticker with TIMER_RELATIVE mode.
    pub fn start_reltime(&self, interval: Duration, from: Duration) -> Result<()> {
        let interval = timespec_t::new(interval.as_secs() as c_long,
                                       interval.subsec_nanos() as c_long);
        let start = timespec_t::new(from.as_secs() as c_long, from.subsec_nanos() as c_long);
        let mut itime = itimerspec_t::with_value(interval, start);
        let rt = unsafe { timer_settime(self.timer_id, 0, &mut itime, 0 as *mut itimerspec_t) };
        if rt != 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    // Stop ticker.
    pub fn stop(&self) -> Result<()> {
        let interval = timespec_t::new(0, 0);
        let start = timespec_t::new(0, 0);
        let mut itime = itimerspec_t::with_value(interval, start);
        let rt = unsafe {
            timer_settime(self.timer_id,
                          TIMER_ABSTIME,
                          &mut itime,
                          0 as *mut itimerspec_t)
        };
        if rt != 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }

    // Get timer id
    pub fn get_id(&self) -> u64 {
        self.timer_id as u64
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if unsafe { timer_delete(self.timer_id) } != 0 {
            panic!("timer_delete failed:{:?}", Error::last_os_error());
        }
    }
}

#[repr(C)]
struct sched_param_t {
    sched_priority: c_int,
}

impl sched_param_t {
    fn new(priority: i32) -> sched_param_t {
        let mut priority = priority;
        let min = unsafe { sched_get_priority_min(SCHED_FIFO) };
        let max = unsafe { sched_get_priority_max(SCHED_FIFO) };
        if priority > max {
            priority = max;
        }
        if priority < min {
            priority = min;
        }
        sched_param_t { sched_priority: priority }
    }
}

#[repr(C)]
struct pthread_attr_t {
    pub __pad: [u8; 56],
}

impl pthread_attr_t {
    fn new() -> pthread_attr_t {
        pthread_attr_t { __pad: [0u8; 56] }
    }
}

#[repr(C)]
struct sigevent_t {
    pub sigev_value: CallbackFunctionParam,
    pub sigev_signo: c_int,
    pub sigev_notify: c_int,
    pub function: extern "C" fn(CallbackFunctionParam),
    pub attribute: *mut pthread_attr_t,
}

impl sigevent_t {
    fn with_callback(cb: extern "C" fn(CallbackFunctionParam),
                     param: CallbackFunctionParam,
                     priority: i32,
                     attr: *mut pthread_attr_t)
                     -> Result<sigevent_t> {
        let sigevent = sigevent_t {
            sigev_value: param,
            sigev_signo: 0,
            sigev_notify: SIGEV_THREAD,
            function: cb,
            attribute: attr,
        };
        let mut sched_param = sched_param_t::new(priority);
        unsafe {
            sched_setscheduler(0, SCHED_FIFO, &mut sched_param);
            pthread_attr_init(sigevent.attribute);
            pthread_attr_setschedparam(sigevent.attribute, &sched_param);
        }
        Ok(sigevent)
    }
}

#[repr(C)]
struct timespec_t {
    pub tv_sec: c_long,
    pub tv_nsec: c_long,
}

impl timespec_t {
    fn new(sec: c_long, nsec: c_long) -> timespec_t {
        timespec_t {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }
}

#[repr(C)]
struct itimerspec_t {
    pub it_interval: timespec_t,
    pub it_value: timespec_t,
}

impl itimerspec_t {
    fn new() -> itimerspec_t {
        itimerspec_t {
            it_interval: timespec_t::new(0, 0),
            it_value: timespec_t::new(0, 0),
        }
    }
    fn with_value(interval: timespec_t, value: timespec_t) -> itimerspec_t {
        itimerspec_t {
            it_interval: interval,
            it_value: value,
        }
    }
}

extern "C" {
    //
    fn sched_get_priority_max(_policy: c_int) -> c_int;
    fn sched_get_priority_min(_policy: c_int) -> c_int;
    fn sched_setscheduler(_pid_t: c_int,
                          _policy: c_int,
                          _sched_param_t: *mut sched_param_t)
                          -> c_int;
    fn pthread_attr_init(_pthread_attr: *mut pthread_attr_t) -> c_int;
    fn pthread_attr_setschedparam(_pthread_attr: *mut pthread_attr_t,
                                  _sched_param_t: *const sched_param_t)
                                  -> c_int;
    //
    fn clock_gettime(_clock_id: c_int, _time_spec: *mut timespec_t) -> c_int;
    fn clock_settime(_clock_id: c_int, _time_spec: *const timespec_t) -> c_int;
    //
    fn timer_create(_clock_id: c_int,
                    _sigevent_t: *mut sigevent_t,
                    _timer_t: *mut TimerId)
                    -> c_int;
    fn timer_delete(_timer_t: TimerId) -> c_int;
    fn timer_settime(_timer_t: TimerId,
                     _flags: c_int,
                     _itimerspec: *const itimerspec_t,
                     _ovalue: *mut itimerspec_t)
                     -> c_int;
    fn timer_getoverrun(_timer_t: TimerId) -> c_int;
    //
    fn adjtime(delta: *mut timeval, olddelta: *mut timeval) -> c_int;
}

pub fn set_os_real_time(sec: i64, nsec: i64) -> Result<()> {
    let spec = timespec_t::new(sec as c_long, nsec as c_long);
    let rt = unsafe { clock_settime(CLOCK_REALTIME, &spec) };
    if rt != 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn get_os_real_time() -> Result<(i64, i64)> {
    let mut spec = timespec_t::new(0, 0);
    let rt = unsafe { clock_gettime(CLOCK_REALTIME, &mut spec) };
    if rt != 0 {
        Err(Error::last_os_error())
    } else {
        Ok((spec.tv_sec as i64, spec.tv_nsec as i64))
    }
}

#[repr(C)]
struct timeval {
    tv_sec: i32,
    tv_usec: i32,
}
// adjust system time with (s, us). return old timeval(s, us)
pub fn adjust_os_time(s: i32, us: i32) -> (i32, i32) {
    let mut ti = timeval {
        tv_sec: s,
        tv_usec: us,
    };

    let mut old = timeval {
        tv_sec: 0,
        tv_usec: 0,
    };

    unsafe {
        // let rt =
        adjtime(&mut ti, &mut old);
        return (old.tv_sec, old.tv_usec);
    }
}

#[test]
fn test_timer() {
    use std::thread;
    use std::time::Duration;
    use std::sync::*;
    let counter = Arc::new(Mutex::new(0));
    let counter1 = counter.clone();
    let mut timer = Timer::new();
    timer.on_arrived.join(move |overrun| {
        println!("counter:{}, overrun:{}", *counter1.lock().unwrap(), overrun);
        *counter1.lock().unwrap() += 1;
    });
    if let Ok(_) = timer.ticker(CLOCK_REALTIME, 50) {
        assert!(timer.get_id() != 0);
        if let Ok(_) = timer.start_reltime(Duration::from_millis(640), Duration::from_secs(1)) {
            thread::sleep(Duration::from_millis(640 * 4 + 100));
        }
    }
    assert!(*counter.lock().unwrap() >= 2);
}