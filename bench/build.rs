use std::process::{Command, Stdio};
use std::env;
use std::fs::{self, File};
use std::io::{self, prelude::*, BufWriter};
use std::path::Path;

use quote::quote;
use proc_macro2::{Ident, Span};

use enum_utils_from_str::StrMapFunc;

struct Dataset {
    name: String,
    values: Vec<String>,
}

impl Dataset {
    fn file(&self, method: &str) -> io::Result<BufWriter<File>> {
        let dir = env::var("OUT_DIR").unwrap();
        let dir = Path::new(&dir);

        let name = format!("{}_{}.rs", self.name, method);
        File::create(&dir.join(name)).map(BufWriter::new)
    }

    fn trie(&self) -> io::Result<()> {
        let mut w = self.file("trie")?;
        let mut t = StrMapFunc::new("_trie", "usize");
        let entries = self.values.iter()
            .enumerate()
            .map(|(i, s)| (&**s, i));
        t.entries(entries);

        let o = quote!{
            pub fn trie(s: &str) -> Option<usize> {
                #t
                _trie(s.as_bytes())
            }
        };
        write!(&mut w, "{}", o)
    }

    fn phf(&self) -> io::Result<()> {
        let mut w = self.file("phf")?;
        write!(&mut w, "static MAP: ::phf::Map<&'static str, usize> = ")?;

        let mut map = phf_codegen::Map::new();
        for (i, atom) in self.values.iter().enumerate() {
            map.entry(atom.as_str(), i.to_string().as_str());
        }

        map.build(&mut w)?;
        writeln!(&mut w, ";")?;

        write!(&mut w, "{}", quote!{
            pub fn phf(s: &str) -> Option<usize> {
                MAP.get(s).cloned()
            }
        })
    }

    fn simple_match(&self) -> io::Result<()> {
        let mut w = self.file("control")?;

        let i = self.values.iter().enumerate().map(|(i, _)| i);
        let values = self.values.iter();
        let out = quote!{
            pub fn control(s: &str) -> Option<usize> {
                match s {
                    #( #values => Some(#i), )*
                    _ => None
                }
            }
        };

        write!(&mut w, "{}", out)
    }

    fn gperf(&self) -> io::Result<()> {
        let name = &self.name;
        let mut w = self.file("gperf")?;

        // Run gperf
        let mut path = env::temp_dir();
        path.push("gperf.c");
        let f = File::create(&path)?;

        let mut child = Command::new("gperf")
            .arg("--includes")
            .arg("--struct-type")
            .arg("--compare-strncmp")
            .args(&["--lookup-function-name", name])
            .stdin(Stdio::piped())
            .stdout(f)
            .spawn()
            .expect("Failed to start gperf");

        {
            let stdin = child.stdin.as_mut().unwrap();
            writeln!(stdin, "struct discriminant {{ char *name; int discriminant; }};")?;
            writeln!(stdin, "%%")?;
            for (i, atom) in self.values.iter().enumerate() {
                writeln!(stdin, "{}, {}", atom, i)?;
            }
        }

        child.wait().unwrap();

        // Compile generated code
        cc::Build::new()
            .file(&path)
            .flag("-Wno-missing-field-initializers")
            .compile(name);

        // Generate wrapper function
        let fname = Ident::new(name, Span::call_site());
        let out = quote!{
            extern crate libc;

            use self::libc::{c_char, c_int, size_t};

            #[repr(C)]
            struct Discriminant {
                name: *const c_char,
                discriminant: c_int,
            }

            #[link(name = #name)]
            extern {
                fn #fname(s: *const c_char, len: size_t) -> *mut Discriminant;
            }

            pub fn gperf(s: &str) -> Option<usize> {
                let discriminant = unsafe {
                    #fname(s.as_ptr() as *const c_char, s.len()).as_ref()
                };

                discriminant.map(|d| {
                    d.discriminant as usize
                })
            }
        };

        write!(w, "{}", out)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut datasets = vec![];
    for entry in fs::read_dir("./data")? {
        let path = entry?.path();
        let name = path.file_stem().unwrap().to_owned().into_string().unwrap();
        let f = io::BufReader::new(File::open(path)?);

        let values: Result<Vec<String>, _> = f.lines()
            .map(|s| s.map(|s| s.trim().to_owned()))
            .collect();

        let data = Dataset { name, values: values? };
        datasets.push(data);
    }

    for data in &datasets {
        data.simple_match()?;
        data.trie()?;
        data.phf()?;

        // gperf fails on large inputs
        if data.values.len() < 2000 {
            data.gperf()?;
        }
    }

    Ok(())
}
