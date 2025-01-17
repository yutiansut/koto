use {
    koto::prelude::*,
    std::{cell::RefCell, rc::Rc},
    wasm_bindgen::prelude::*,
};

// Captures output from Koto in a String
#[derive(Debug)]
struct OutputCapture {
    output: Rc<RefCell<String>>,
}

impl KotoFile for OutputCapture {
    fn id(&self) -> ValueString {
        "_stdout_".into()
    }
}

impl KotoRead for OutputCapture {}
impl KotoWrite for OutputCapture {
    fn write(&self, bytes: &[u8]) -> Result<(), RuntimeError> {
        let bytes_str = match std::str::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string().into()),
        };
        self.output.borrow_mut().push_str(bytes_str);
        Ok(())
    }

    fn write_line(&self, output: &str) -> Result<(), RuntimeError> {
        let mut unlocked = self.output.borrow_mut();
        unlocked.push_str(output);
        unlocked.push('\n');
        Ok(())
    }

    fn flush(&self) -> Result<(), RuntimeError> {
        Ok(())
    }
}

struct BlockedInput {}

impl KotoFile for BlockedInput {
    fn id(&self) -> ValueString {
        "_stdin_".into()
    }
}

impl KotoWrite for BlockedInput {}
impl KotoRead for BlockedInput {
    fn read_line(&self) -> Result<Option<String>, RuntimeError> {
        runtime_error!("Unsupported in the browser")
    }

    fn read_to_string(&self) -> Result<String, RuntimeError> {
        runtime_error!("Unsupported in the browser")
    }
}

// Runs an input program and returns the output as a String
#[wasm_bindgen]
pub fn compile_and_run(input: &str) -> String {
    let output = Rc::new(RefCell::new(String::new()));

    let mut koto = Koto::with_settings(
        KotoSettings::default()
            .with_stdin(BlockedInput {})
            .with_stdout(OutputCapture {
                output: output.clone(),
            })
            .with_stderr(OutputCapture {
                output: output.clone(),
            }),
    );

    match koto.compile(input) {
        Ok(_) => match koto.run() {
            Ok(_) => std::mem::take(&mut output.borrow_mut()),
            Err(error) => format!("Runtime error: {error}"),
        },
        Err(error) => format!("Compilation error: {error}"),
    }
}
