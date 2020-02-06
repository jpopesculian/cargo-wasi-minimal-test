#[macro_use]
extern crate anyhow;

use anyhow::Result;
use anyhow::{bail, format_err};
use std::convert::TryInto;
use std::{fs, path};
use wasmtime::{Config, Engine, Extern, Instance, Module, Store};
use wasmtime_interface_types::ModuleData;

pub struct WasmLib {
    instance: Instance,
    data: ModuleData,
}

impl WasmLib {
    pub fn load_file(file: impl AsRef<path::Path>) -> Result<Self> {
        Self::load_bytes(&fs::read(file)?)
    }

    pub fn load_bytes(bytes: impl AsRef<[u8]>) -> Result<Self> {
        let engine = Engine::new(
            Config::new()
                .wasm_multi_value(true)
                .wasm_reference_types(true)
                .debug_info(true),
        );
        let store = Store::new(&engine);
        let data = ModuleData::new(bytes.as_ref())?;
        let module = Module::new(&store, bytes.as_ref())?;

        let mut imports: Vec<Extern> = Vec::new();
        if let Some(module_name) = data.find_wasi_module_name() {
            let wasi_instance = wasmtime_wasi::create_wasi_instance(&store, &[], &[], &[])
                .map_err(|e| format_err!("wasm instantiation error: {:?}", e))?;
            for i in module.imports().iter() {
                if i.module() != module_name {
                    bail!("unknown import module {}", i.module());
                }
                if let Some(export) = wasi_instance.get_export(i.name()) {
                    imports.push(export.clone());
                } else {
                    bail!("unknown import {}:{}", i.module(), i.name())
                }
            }
        }

        let instance = Instance::new(&module, &imports)
            .map_err(|t| format_err!("instantiation trap: {:?}", t))?;

        Ok(Self { instance, data })
    }

    pub fn add(&self, a: i32, b: i32) -> Result<i32> {
        let mut results = self
            .data
            .invoke_export(&self.instance, "add", &[a.into(), b.into()])?;
        Ok(results
            .pop()
            .ok_or_else(|| anyhow!("no return"))?
            .try_into()?)
    }
}

fn main() -> Result<()> {
    let lib = WasmLib::load_file("../target/wasm32-wasi/release/wasm_lib.wasm")?;
    let sum = lib.add(1, 2)?;
    println!("{:?}", sum);
    Ok(())
}
