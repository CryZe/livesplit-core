use crate::environment::{Environment, Imports};
use crate::pointer::PointerValue;
use crate::process::{Offset, Process};
use std::{error::Error, mem, thread};
use wasmi::{
    ExternVal, FuncInstance, FuncRef, MemoryRef, Module, ModuleInstance, ModuleRef, RuntimeValue,
};

pub struct Runtime {
    _instance: ModuleRef,
    environment: Environment,
    timer_state: TimerState,
    should_start: Option<FuncRef>,
    should_split: Option<FuncRef>,
    should_reset: Option<FuncRef>,
    is_loading: Option<FuncRef>,
    game_time: Option<FuncRef>,
    update: Option<FuncRef>,
    disconnected: Option<FuncRef>,
    is_loading_val: Option<bool>,
    game_time_val: Option<f64>,
}

#[repr(u8)]
pub enum TimerState {
    NotRunning = 0,
    Running = 1,
    Finished = 2,
}

#[derive(Debug)]
pub enum TimerAction {
    Start,
    Split,
    Reset,
}

impl Runtime {
    pub fn new(binary: &[u8]) -> Result<Self, Box<Error>> {
        let module = Module::from_buffer(binary)?;
        let instance = ModuleInstance::new(&module, &Imports)?;
        let memory = into_memory(
            instance
                .not_started_instance()
                .export_by_name("memory")
                .ok_or("memory not exported")?,
        )?;
        let mut environment = Environment::new(memory);
        let instance = instance.run_start(&mut environment)?;
        instance.invoke_export("configure", &[], &mut environment)?;

        let should_start = instance
            .export_by_name("should_start")
            .and_then(|e| e.as_func()?.clone().into());
        let should_split = instance
            .export_by_name("should_split")
            .and_then(|e| e.as_func()?.clone().into());
        let should_reset = instance
            .export_by_name("should_reset")
            .and_then(|e| e.as_func()?.clone().into());
        let is_loading = instance
            .export_by_name("is_loading")
            .and_then(|e| e.as_func()?.clone().into());
        let game_time = instance
            .export_by_name("game_time")
            .and_then(|e| e.as_func()?.clone().into());
        let update = instance
            .export_by_name("update")
            .and_then(|e| e.as_func()?.clone().into());
        let disconnected = instance
            .export_by_name("disconnected")
            .and_then(|e| e.as_func()?.clone().into());

        Ok(Self {
            _instance: instance,
            environment,
            timer_state: TimerState::NotRunning,
            should_start,
            should_split,
            should_reset,
            is_loading,
            game_time,
            update,
            disconnected,
            is_loading_val: None,
            game_time_val: None,
        })
    }

    pub fn sleep(&self) {
        thread::sleep(self.environment.tick_rate);
    }

    pub fn step(&mut self) -> Result<Option<TimerAction>, Box<Error>> {
        let mut just_connected = false;
        if self.environment.process.is_none() {
            self.environment.process = match Process::with_name(&self.environment.process_name) {
                Ok(p) => Some(p),
                Err(_) => return Ok(None),
            };
            eprintln!("Connected");
            just_connected = true;
        }

        if self.update_values(just_connected).is_err() {
            // TODO: Only checks for disconnected if we actually have pointer paths
            eprintln!("Disconnected");
            self.environment.process = None;
            if let Some(func) = &self.disconnected {
                FuncInstance::invoke(func, &[], &mut self.environment)?;
            }
            return Ok(None);
        }
        // println!("{:#?}", self.environment);
        self.run_script()
    }

    pub fn set_state(&mut self, state: TimerState) {
        self.timer_state = state;
    }

    fn update_values(&mut self, just_connected: bool) -> Result<(), Box<Error>> {
        let process = self
            .environment
            .process
            .as_mut()
            .expect("The process should be connected at this point");

        for pointer_path in &mut self.environment.pointer_paths {
            let mut address = process.module_address(&pointer_path.module_name)?;
            let mut offsets = pointer_path.offsets.iter().cloned().peekable();
            if process.is_64bit() {
                while let Some(offset) = offsets.next() {
                    address = (address as Offset).wrapping_add(offset) as u64;
                    if offsets.peek().is_some() {
                        address = process.read(address)?;
                    }
                }
            } else {
                while let Some(offset) = offsets.next() {
                    address = (address as i32).wrapping_add(offset as i32) as u64;
                    if offsets.peek().is_some() {
                        address = process.read::<u32>(address)? as u64;
                    }
                }
            }
            match &mut pointer_path.old {
                PointerValue::U8(v) => *v = process.read(address)?,
                PointerValue::U16(v) => *v = process.read(address)?,
                PointerValue::U32(v) => *v = process.read(address)?,
                PointerValue::U64(v) => *v = process.read(address)?,
                PointerValue::I8(v) => *v = process.read(address)?,
                PointerValue::I16(v) => *v = process.read(address)?,
                PointerValue::I32(v) => *v = process.read(address)?,
                PointerValue::I64(v) => *v = process.read(address)?,
                PointerValue::F32(v) => *v = process.read(address)?,
                PointerValue::F64(v) => *v = process.read(address)?,
                PointerValue::String(_) => unimplemented!(),
            }
        }

        if just_connected {
            for pointer_path in &mut self.environment.pointer_paths {
                pointer_path.current.clone_from(&pointer_path.old);
            }
        } else {
            for pointer_path in &mut self.environment.pointer_paths {
                mem::swap(&mut pointer_path.current, &mut pointer_path.old);
            }
        }

        Ok(())
    }

    fn run_script(&mut self) -> Result<Option<TimerAction>, Box<Error>> {
        if let Some(func) = &self.update {
            FuncInstance::invoke(func, &[], &mut self.environment)?;
        }

        match &self.timer_state {
            TimerState::NotRunning => {
                if let Some(func) = &self.should_start {
                    let ret_val = FuncInstance::invoke(func, &[], &mut self.environment)?;

                    if let Some(RuntimeValue::I32(1)) = ret_val {
                        return Ok(Some(TimerAction::Start));
                    }
                }
            }
            TimerState::Running => {
                if let Some(func) = &self.is_loading {
                    let ret_val = FuncInstance::invoke(func, &[], &mut self.environment)?;

                    self.is_loading_val = match ret_val {
                        Some(RuntimeValue::I32(val)) => Some(val != 0),
                        _ => None,
                    };
                }
                if let Some(func) = &self.game_time {
                    let ret_val = FuncInstance::invoke(func, &[], &mut self.environment)?;

                    self.game_time_val = match ret_val {
                        Some(RuntimeValue::F64(val)) => {
                            let val = val.to_float();
                            if val.is_nan() {
                                None
                            } else {
                                Some(val)
                            }
                        }
                        _ => None,
                    };
                }

                if let Some(func) = &self.should_split {
                    let ret_val = FuncInstance::invoke(func, &[], &mut self.environment)?;

                    if let Some(RuntimeValue::I32(1)) = ret_val {
                        return Ok(Some(TimerAction::Split));
                    }
                }
                if let Some(func) = &self.should_reset {
                    let ret_val = FuncInstance::invoke(func, &[], &mut self.environment)?;

                    if let Some(RuntimeValue::I32(1)) = ret_val {
                        return Ok(Some(TimerAction::Reset));
                    }
                }
            }
            TimerState::Finished => {
                if let Some(func) = &self.should_reset {
                    let ret_val = FuncInstance::invoke(func, &[], &mut self.environment)?;

                    if let Some(RuntimeValue::I32(1)) = ret_val {
                        return Ok(Some(TimerAction::Reset));
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn is_loading(&self) -> Option<bool> {
        self.is_loading_val
    }

    pub fn game_time(&self) -> Option<f64> {
        self.game_time_val
    }
}

fn into_memory(extern_val: ExternVal) -> Result<MemoryRef, Box<Error>> {
    match extern_val {
        ExternVal::Memory(memory) => Ok(memory),
        _ => Err("Memory is not exported correctly".into()),
    }
}
