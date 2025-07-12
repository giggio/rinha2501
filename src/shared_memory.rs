use std::io::{self, Write};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use raw_sync::locks::*;
use shared_memory::*;

use initializer::ShmemInitializer;

pub struct SharedMemory<T> {
    pub mutex: MutexWrapper<T>,
    #[allow(dead_code)] shmem: Arc<std::sync::Mutex<Shmem>>, // need to hold a reference to shmem or the mutex will fail as the shmem is deleted on drop
}

pub struct MutexWrapper<T>(Mutex, PhantomData<T>);

impl<T> SharedMemory<T> {
    pub fn new(shmem_flink: &str) -> Result<SharedMemory<T>, String> {
        ShmemInitializer::new(shmem_flink)?.create_shmem()
    }

    #[allow(dead_code)]
    pub fn is_initialized(shmem_flink: &str) -> Result<bool, String> {
        ShmemInitializer::new(shmem_flink)?.is_initialized()
    }
}

impl<T> MutexWrapper<T> {
    #[allow(dead_code)]
    pub fn get(&self) -> Result<&T, String> {
        match self.0.lock().ok() {
            Ok(guard) => {
                let val = unsafe { &*(*guard as *const u8 as *const T) };
                Ok(&*val)
            }
            Err(_) => {
                Err("Failed to acquire lock, returning default value.".to_string())
            }
        }
    }

    pub fn get_projection<F: Fn(&T) -> R, R>(&self, f: F) -> Result<R, String> {
        match self.0.lock().ok() {
            Ok(guard) => {
                let val = unsafe { &*(*guard as *const u8 as *const T) };
                let projection = f(&*val);
                Ok(projection)
            }
            Err(_) => {
                Err("Failed to acquire lock, returning default value.".to_string())
            }
        }
    }

    pub fn set(&self, value: T) -> Result<(), String> {
        let guard = self.0.lock().ok().map_err(|e| format!("Failed to acquire lock: {}", e))?;
        let val = unsafe { &mut *(*guard as *const u8 as *mut T) };
        *val = value;
        Ok(())
    }

    pub fn set_fn<F: Fn(&mut T)>(&self, update: F) -> Result<(), String> {
        let guard = self.0.lock().ok().map_err(|e| format!("Failed to acquire lock: {}", e))?;
        let val = unsafe { &mut *(*guard as *const u8 as *mut T) };
        update(val);
        Ok(())
    }
}


mod initializer {
    use super::*;

    pub struct ShmemInitializer<'a> {
        shmem: Shmem,
        is_init: &'a mut AtomicBool,
        raw_ptr: *mut u8,
    }

    impl<'a> ShmemInitializer<'a> {
        pub fn new(shmem_flink: &str) -> Result<ShmemInitializer, String> {
            let shmem = match ShmemConf::new().size(4096).flink(shmem_flink).create() {
                Ok(m) => m,
                Err(ShmemError::LinkExists) => ShmemConf::new().flink(shmem_flink).open().map_err(|e| {
                    format!("Unable to open shmem flink {shmem_flink} : {e}")
                })?,
                Err(e) => {
                    eprintln!("Unable to create or open shmem flink {shmem_flink} : {e}");
                    return Err(format!("Unable to create or open shmem flink {shmem_flink} : {e}"));
                }
            };

            let mut raw_ptr = shmem.as_ptr();
            let is_init: &mut AtomicBool;

            unsafe {
                is_init = &mut *(raw_ptr as *mut u8 as *mut AtomicBool);
                raw_ptr = raw_ptr.add(size_of::<AtomicBool>() * 8);
            };
            Ok(ShmemInitializer {
                shmem,
                is_init,
                raw_ptr
            })
        }

        pub fn is_initialized(&self) -> Result<bool, String> {
            if self.shmem.is_owner() {
                println!("This is a new owner of the shared memory, so it was not initialized elsewhere.");
                return Ok(false);
            }
            if !self.is_init.load(Ordering::Acquire) {
                print!("Waiting for mutex to be initialized...");
                let mut times = 0;
                while !self.is_init.load(Ordering::Acquire) {
                    times += 1;
                    if times > 10 {
                        println!("failed!");
                        println!("Giving up after 1 second of waiting for mutex to be initialized");
                        return Ok(false);
                    }
                    print!(".");
                    io::stdout().flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;  // Ensure the prompt is printed before reading input
                    sleep(Duration::from_millis(100));
                    std::thread::yield_now();
                }
                println!("done!");
            }
            return Ok(true);
        }

        pub fn create_mutex(&self) -> Result<Mutex, String> {
            let mutex = if self.shmem.is_owner() {
                println!("This is the owner of the shared memory, initializing mutex");
                // Initialize the mutex
                let (lock, _bytes_used) = unsafe {
                    Mutex::new(
                        self.raw_ptr,                                    // Base address of Mutex
                        self.raw_ptr.add(Mutex::size_of(Some(self.raw_ptr))), // Address of data protected by mutex
                    ).map_err(|e| format!("Failed to create mutex: {}", e))?
                };
                println!("Mutex initialized, waiting for 5 seconds to view initialization...");
                sleep(Duration::from_secs(5));
                self.is_init.store(true, Ordering::Release);
                // let any:Box<dyn std::any::Any> = lock;
                lock
            } else {
                println!("This is not the owner of the shared memory, waiting for mutex to be initialized");
                if !self.is_init.load(Ordering::Acquire) {
                    print!("Waiting for mutex to be initialized...");
                    while !self.is_init.load(Ordering::Acquire) {
                        print!(".");
                        io::stdout().flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;  // Ensure the prompt is printed before reading input
                        sleep(Duration::from_millis(100));
                        std::thread::yield_now();
                    }
                    println!("done!");
                }
                let (lock, _bytes_used) = unsafe {
                    Mutex::from_existing(
                        self.raw_ptr,                                    // Base address of Mutex
                        self.raw_ptr.add(Mutex::size_of(Some(self.raw_ptr))), // Address of data  protected by mutex
                    ).map_err(|e| format!("Failed to create mutex: {}", e))?
                };
                lock
            };
            Ok(mutex)
        }

        pub fn create_shmem<T>(self) -> Result<SharedMemory<T>, String> {
            let mutex = self.create_mutex()?;
            Ok(SharedMemory::<T> {
                mutex: MutexWrapper(mutex, PhantomData),
                shmem: Arc::new(std::sync::Mutex::new(self.shmem)),
                // phantom: PhantomData,
            })
        }
    }
}
