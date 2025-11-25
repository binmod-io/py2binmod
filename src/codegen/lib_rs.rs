use proc_macro2::{TokenStream, Span};
use quote::quote;
use syn::Ident;

use crate::{
    types::{ProjectContext, ParameterType, ModuleFunction}, 
    codegen::traits::{CodeGenerator, AsTokenStream},
};


pub struct LibRsGenerator {
    context: ProjectContext,
}

impl LibRsGenerator {
    pub fn new(context: ProjectContext) -> Self {
        Self { context }
    }

    fn generate_imports(&self) -> TokenStream {
        quote! {
            use serde_json::value::Serializer;
            use serde::{Serialize, de::DeserializeOwned};
            use rustpython_vm::{
                Interpreter,
                VirtualMachine,
                PyObjectRef,
                PyResult,
                AsObject,
                py_freeze,
                pymodule,
                builtins::PyBaseExceptionRef,
                convert::ToPyObject,
                py_serde::{serialize, deserialize},
            };
            use rustpython_stdlib::get_module_inits;
            use rustpython_pylib::FROZEN_STDLIB;
            use binmod_mdk::{host_fns, mod_fn, FnResult, ModuleFnErr};
        }
    }

    fn generate_utils(&self) -> TokenStream {
        quote! {
            fn rs_to_py<T: Serialize>(vm: &VirtualMachine, value: T) -> FnResult<PyObjectRef> {
                let serialized = serde_json::to_value(&value)
                    .map_err(|exc| ModuleFnErr {
                        error_type: "SerializationError".to_string(),
                        message: format!("Failed to serialize: {}", exc),
                    })?;
                let py_obj = deserialize(vm, serialized)
                    .map_err(|exc| ModuleFnErr {
                        error_type: "DeserializationError".to_string(),
                        message: format!("Failed to deserialize: {}", exc),
                    })?;

                Ok(py_obj)
            }

            fn py_to_rs<T: DeserializeOwned>(vm: &VirtualMachine, obj: PyObjectRef) -> FnResult<T> {
                let serialized = serialize(vm, obj.as_object(), Serializer)
                    .map_err(|exc| ModuleFnErr {
                        error_type: "SerializationError".to_string(),
                        message: format!("Failed to serialize: {}", exc),
                    })?;
                let deserialized = serde_json::from_value::<T>(serialized)
                    .map_err(|exc| ModuleFnErr {
                        error_type: "DeserializationError".to_string(),
                        message: format!("Failed to deserialize: {}", exc),
                    })?;

                Ok(deserialized)
            }


            pub fn from_py_exc(vm: &VirtualMachine, exc: PyBaseExceptionRef) -> ModuleFnErr {
                let mut buffer = String::new();
                vm
                    .write_exception(&mut buffer, &exc)
                    .unwrap();

                ModuleFnErr {
                    error_type: exc.class().to_string(),
                    message: buffer,
                }
            }


            pub fn to_py_exc(vm: &VirtualMachine, err: ModuleFnErr) -> PyBaseExceptionRef {
                vm.new_runtime_error(
                    format!("Error in Python module: {}: {}", err.error_type, err.message),
                )
            }
        }
    }

    fn generate_globals(&self) -> TokenStream {
        let module_dir_str = self.context.module_root.parent().unwrap().to_string_lossy();
        let site_packages_dir_str = self.context.site_packages_dir.to_string_lossy();

        quote! {
            thread_local! {
                static INTERPRETER: Interpreter = Interpreter::with_init(Default::default(), |vm| {
                    vm.add_native_modules(get_module_inits());
                    vm.add_native_module("hostfns", Box::new(hostfns::make_module));
                    vm.add_frozen(FROZEN_STDLIB);
                    vm.add_frozen(py_freeze!(dir = #module_dir_str));
                    vm.add_frozen(py_freeze!(dir = #site_packages_dir_str));
                });
            }
        }
    }

    fn generate_host_functions(&self) -> TokenStream {
        let host_functions = match self.context
            .modules
            .iter()
            .find_map(|module| module.host_functions.as_ref())
        {
            Some(host_fns) => host_fns,
            None => return quote! {},
        };

        let namespace = &host_functions.namespace;

        let extern_fns = host_functions
            .iter()
            .map(|f| {
                let name: Ident = Ident::new(&f.name, Span::call_site());
                let params = f.parameters
                    .iter()
                    .map(|p| p.as_token_stream());
                let return_type = f.return_type.as_token_stream();

                quote! {
                    fn #name(#(#params),*) -> #return_type;
                }
            });

        let wrappers = host_functions
            .iter()
            .map(|f| {
                let fn_name = Ident::new(&format!("{}_wrapper", &f.name), Span::call_site());
                let fn_name_str = &f.name;
                let host_fn_name = Ident::new(&f.name, Span::call_site());
                let params = f.parameters
                    .iter()
                    .map(|p| p.as_token_stream());
                let param_names = f.parameters
                    .iter()
                    .filter_map(|p| p.as_token_stream().to_string()
                        .split(':')
                        .next()
                        .map(|s| Ident::new(s.trim(), Span::call_site()))
                    );
                let return_type = f.return_type.as_token_stream();

                match params.len() {
                    0 => quote! {
                        #[pyfunction(name = #fn_name_str)]
                        fn #fn_name(vm: &VirtualMachine) -> PyResult<#return_type> {
                            unsafe { #host_fn_name() }
                                .map_err(|err| to_py_exc(vm, err))
                        }
                    },
                    _ => quote! {
                        #[pyfunction(name = #fn_name_str)]
                        fn #fn_name(#(#params),*, vm: &VirtualMachine) -> PyResult<#return_type> {
                            unsafe { #host_fn_name(#(#param_names),*) }
                                .map_err(|err| to_py_exc(vm, err))
                        }
                    },
                }
            });

        quote! {
            #[host_fns(namespace = #namespace)]
            unsafe extern "host" {
                #(#extern_fns)*
            }

            #[pymodule]
            mod hostfns {
                use super::*;

                #(#wrappers)*
            }

        }

    }

    fn generate_initialize(&self) -> TokenStream {
        let namespace = self.context
            .modules
            .iter()
            .find(|module| module.host_functions.is_some())
            .map(|module| module.host_functions.as_ref().unwrap().namespace.clone())
            .unwrap_or("env".to_string());

        quote! {
            #[mod_fn(name = "initialize")]
            pub fn initialize_impl() -> FnResult<()> {
                INTERPRETER.with(|interpreter| {
                    interpreter.enter(|vm| {
                        vm.import("binmod_mdk", 0)
                            .and_then(|py_binmod_mdk| {
                                py_binmod_mdk.get_attr("_register_host_fns", vm)
                            })
                            .and_then(|register_fn| {
                                vm.import("hostfns", 0)
                                    .map(|py_hostfns| (register_fn, py_hostfns))
                            })
                            .and_then(|(register_fn, py_hostfns)| {
                                register_fn.call((#namespace.to_pyobject(vm), py_hostfns.as_object()), vm)
                            })
                            .map_err(|exc| from_py_exc(vm, exc))
                    })
                })?;

                Ok(())
            }
        }
    }

    fn generate_exported_functions(&self) -> TokenStream {
        let functions = self.context
            .modules
            .iter()
            .flat_map(|module| module.module_functions
                .iter()
                .map(move |f| (module, f))
            )
            .map(|(module, func)| self
                .generate_exported_function_shim(
                    func, 
                    &module.import_path(&self.context.module_root)
                        .map(|s| format!("{}.{}", self.context.module_name, s))
                        .unwrap_or_else(|| self.context.module_name.clone())
                        .as_str(),
                )
            )
            .collect::<Vec<TokenStream>>();

        quote! {
            #(#functions)*
        }
    }

    fn generate_exported_function_shim(&self, func: &ModuleFunction, import_path: &str) -> TokenStream {
        let fn_impl_name = Ident::new(&format!("{}_shim", &func.name), Span::call_site());
        let mod_fn_name = &func.name;
        let docstring = func.docstring
            .as_deref()
            .unwrap_or("");
        let parameters = func.parameters
            .iter()
            .map(|p| p.as_token_stream());
        let return_type = func.return_type.as_token_stream();

        let body = match func.return_type {
            ParameterType::None => {
                self.generate_exported_function_shim_unit_body(
                    fn_impl_name,
                    mod_fn_name,
                    import_path,
                    docstring,
                    parameters
                )
            }
            _ => {
                self.generate_exported_function_shim_body(
                    fn_impl_name,
                    mod_fn_name,
                    import_path,
                    docstring,
                    parameters,
                    return_type
                )
            }
        };

        quote! {
            #body
        }
    }

    fn generate_exported_function_shim_body(
        &self,
        fn_impl_name: Ident,
        mod_fn_name: &str,
        import_path: &str,
        docstring: &str,
        parameters: impl Iterator<Item = TokenStream>,
        return_type: TokenStream,
    ) -> TokenStream {
        let params = parameters.collect::<Vec<TokenStream>>();
        let param_names = params
            .iter()
            .filter_map(|p| p.to_string()
                .split(':')
                .next()
                .map(|s| Ident::new(s.trim(), Span::call_site()))
            )
            .collect::<Vec<Ident>>();
        let call_args = match param_names.len() {
            0 => quote! { () },
            1 => {
                let first = &param_names[0];
                quote! { (rs_to_py(vm, #first)?,) }
            },
            _ => quote! { (#(rs_to_py(vm, #param_names)?),*) },
        };

        quote! {
            #[doc = #docstring]
            #[mod_fn(name = #mod_fn_name)]
            pub fn #fn_impl_name(#(#params),*) -> FnResult<#return_type> {
                INTERPRETER.with(|interpreter| {
                    interpreter.enter(|vm| {
                        Ok(
                            py_to_rs::<#return_type>(
                                vm,
                                vm.import(#import_path, 0)
                                    .map_err(|exc| from_py_exc(vm, exc))?
                                    .get_attr(#mod_fn_name, vm)
                                    .map_err(|exc| from_py_exc(vm, exc))?
                                    .call(#call_args, vm)
                                    .map_err(|exc| from_py_exc(vm, exc))?
                            )?
                        )
                    })
                })
            }
        }
    }

    fn generate_exported_function_shim_unit_body(
        &self,
        fn_impl_name: Ident,
        mod_fn_name: &str,
        import_path: &str,
        docstring: &str,
        parameters: impl Iterator<Item = TokenStream>,
    ) -> TokenStream {
        let params = parameters.collect::<Vec<TokenStream>>();
        let param_names = params
            .iter()
            .filter_map(|p| p.to_string()
                .split(':')
                .next()
                .map(|s| Ident::new(s.trim(), Span::call_site()))
            )
            .collect::<Vec<Ident>>();
        let call_args = match param_names.len() {
            0 => quote! { () },
            1 => {
                let first = &param_names[0];
                quote! { (rs_to_py(vm, #first)?,) }
            },
            _ => quote! { (#(rs_to_py(vm, #param_names)?),*) },
        };

        quote! {
            #[doc = #docstring]
            #[mod_fn(name = #mod_fn_name)]
            pub fn #fn_impl_name(#(#params),*) -> FnResult<()> {
                INTERPRETER.with(|interpreter| {
                    interpreter.enter(|vm| {
                        vm.import(#import_path, 0)
                            .map_err(|exc| from_py_exc(vm, exc))?
                            .get_attr(#mod_fn_name, vm)
                            .map_err(|exc| from_py_exc(vm, exc))?
                            .call(#call_args, vm)
                            .map_err(|exc| from_py_exc(vm, exc))?;

                        Ok(())
                    })
                })
            }
        }
    }
}

impl CodeGenerator for LibRsGenerator {
    fn generate(&self) -> TokenStream {
        let globals = self.generate_globals();
        let imports = self.generate_imports();
        let utils = self.generate_utils();
        let host_functions = self.generate_host_functions();
        let initialize = self.generate_initialize();
        let exported_functions = self.generate_exported_functions();

        quote! {
            #imports

            #utils

            #globals

            #host_functions

            #initialize

            #exported_functions
        }
    }
}