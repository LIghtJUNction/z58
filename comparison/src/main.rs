use std::{
    env, fs,
    hint::black_box,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use z58_comparison::*;

struct Run {
    csv: BufWriter<fs::File>,
    smoke: bool,
}

impl Run {
    fn measure<F: FnMut(usize) -> usize>(
        &mut self,
        algorithm: &str,
        operation: &str,
        bytes: usize,
        pattern: &str,
        mut f: F,
    ) {
        let probe = (1024 / bytes.max(1)).clamp(1, 16);
        // Warm-up and calibration do not contribute to the reported samples.
        for i in 0..probe * 2 {
            black_box(f(i % CORPUS));
        }
        let start = Instant::now();
        for i in 0..probe {
            black_box(f(i % CORPUS));
        }
        let elapsed = start.elapsed().as_nanos().max(1);
        let target = if self.smoke { 50_000 } else { 2_000_000 };
        let iterations = ((target * probe as u128 / elapsed) as usize).clamp(1, 1_000_000);
        for sample in 0..if self.smoke { 2 } else { 9 } {
            let start = Instant::now();
            let mut observed = 0usize;
            for i in 0..iterations {
                observed = observed.wrapping_add(f((i + sample) % CORPUS));
            }
            let ns = start.elapsed().as_nanos();
            black_box(observed);
            writeln!(
                self.csv,
                "{algorithm},{operation},{bytes},{pattern},{sample},{iterations},{ns},{:.6}",
                ns as f64 / iterations as f64
            )
            .unwrap();
        }
    }

    fn encode<F: FnMut(&[u8], &mut [u8]) -> usize>(
        &mut self,
        name: &str,
        bytes: usize,
        pattern: &str,
        inputs: &[Vec<u8>],
        expected: &[Vec<u8>],
        mut f: F,
    ) {
        let capacity = z58::encoded_capacity(bytes).unwrap().max(89);
        let mut out = vec![0xa5; capacity];
        for (input, answer) in inputs.iter().zip(expected) {
            let n = f(input, &mut out);
            assert_eq!(&out[..n], answer, "{name} encode {bytes} {pattern}");
        }
        self.measure(name, "encode", bytes, pattern, |i| {
            let n = f(black_box(&inputs[i]), black_box(&mut out));
            black_box(&out[..n]);
            n
        });
        // Revalidate after measurements so mutations or stale output cannot pass.
        for (input, answer) in inputs.iter().zip(expected) {
            let n = f(input, &mut out);
            assert_eq!(&out[..n], answer, "post-timing {name}");
        }
    }

    fn decode<F: FnMut(&[u8], &mut [u8]) -> usize>(
        &mut self,
        name: &str,
        bytes: usize,
        pattern: &str,
        encoded: &[Vec<u8>],
        expected: &[Vec<u8>],
        mut f: F,
    ) {
        let mut out = vec![0xa5; z58::encoded_capacity(bytes).unwrap().max(89)];
        for (input, answer) in encoded.iter().zip(expected) {
            let n = f(input, &mut out);
            assert_eq!(&out[..n], answer, "{name} decode {bytes} {pattern}");
        }
        self.measure(name, "decode", bytes, pattern, |i| {
            let n = f(black_box(&encoded[i]), black_box(&mut out));
            black_box(&out[..n]);
            n
        });
        for (input, answer) in encoded.iter().zip(expected) {
            let n = f(input, &mut out);
            assert_eq!(&out[..n], answer, "post-timing {name}");
        }
    }
}

fn command(cmd: &str, args: &[&str]) -> String {
    match Command::new(cmd)
        .args(args)
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap())
        .output()
    {
        Ok(out) => format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => format!("unavailable: {e}\n"),
    }
}

fn collect_sources(path: &Path, out: &mut Vec<PathBuf>) {
    for item in fs::read_dir(path).unwrap() {
        let item = item.unwrap().path();
        if item.is_dir() {
            collect_sources(&item, out);
        } else if item.extension().is_some_and(|ext| ext == "rs") {
            out.push(item);
        }
    }
}

fn metadata(directory: &Path, smoke: bool) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut text = format!(
        "z58 benchmark\nmode={}\nseed={SEED:#018x}\ncorpus_per_case={CORPUS}\nlengths={LENGTHS:?}\npatterns={PATTERNS:?}\nsamples={}\ntarget_sample_ns={}\nRUSTFLAGS={}\nCC={}\nCFLAGS={}\ncompile_arch={}\ncompile_avx2={}\ncompile_avx512f={}\ncompile_avx512ifma={}\ncompile_avx512vbmi={}\n",
        if smoke {
            "smoke; not performance evidence"
        } else {
            "full"
        },
        if smoke { 2 } else { 9 },
        if smoke { 50_000 } else { 2_000_000 },
        env::var("RUSTFLAGS").unwrap_or_default(),
        env::var("CC").unwrap_or_default(),
        env::var("CFLAGS").unwrap_or_default(),
        env::consts::ARCH,
        cfg!(target_feature = "avx2"),
        cfg!(target_feature = "avx512f"),
        cfg!(target_feature = "avx512ifma"),
        cfg!(target_feature = "avx512vbmi"),
    );
    for (name, args) in [
        ("rustc", vec!["-Vv"]),
        ("cargo", vec!["-V"]),
        ("cc", vec!["--version"]),
        ("uname", vec!["-a"]),
        ("lscpu", vec![]),
        ("git", vec!["rev-parse", "HEAD"]),
        ("git", vec!["status", "--porcelain"]),
    ] {
        text.push_str(&format!(
            "\n$ {name} {}\n{}",
            args.join(" "),
            command(name, &args)
        ));
    }
    for path in [
        "/etc/os-release",
        "/proc/self/status",
        "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor",
    ] {
        if let Ok(value) = fs::read_to_string(path) {
            text.push_str(&format!("\n{path}\n{value}"));
        }
    }
    if let Ok(cpu) = fs::read_to_string("/proc/cpuinfo") {
        text.push_str("\n/proc/cpuinfo (first processor)\n");
        text.push_str(cpu.split("\n\n").next().unwrap_or(&cpu));
        text.push('\n');
    }
    let mut sources = Vec::new();
    for directory in [root.join("../src"), root.join("src"), root.join("tests")] {
        collect_sources(&directory, &mut sources);
    }
    for path in [
        root.join("../Cargo.toml"),
        root.join("Cargo.toml"),
        root.join("Cargo.lock"),
    ] {
        sources.push(path);
    }
    sources.sort();
    if let Ok(hashes) = Command::new("sha256sum").args(&sources).output() {
        text.push_str("\nMeasured-source SHA-256 hashes\n");
        text.push_str(&String::from_utf8_lossy(&hashes.stdout));
    }
    text.push_str("\nProfile: opt-level=3, lto=thin, codegen-units=1 for every Rust dependency. Braid58's upstream build.rs compiles its C kernels at the Cargo optimization level and selects its fastest supported backend from target features. No inputs, output allocation, scratch allocation or verification are timed. Each implementation is verified on all 16 inputs before and after samples. Warmup and adaptive calibration precede samples. Function call, buffer/length checks, strict validation and black_box overhead are timed. Fixed inputs share buffers with generic tests. fd-bs58 encode is not timed because it allocates String. Shared-host timings are exploratory, not an acceptance gate.\n");
    fs::write(directory.join("metadata.txt"), text).unwrap();
    fs::copy(root.join("Cargo.lock"), directory.join("Cargo.lock")).unwrap();
    fs::copy(
        root.join("baselines.json"),
        directory.join("baselines.json"),
    )
    .unwrap();
}

fn main() {
    let mut smoke = false;
    let mut output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("results/latest");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke" => smoke = true,
            "--output" => output = args.next().expect("--output needs a directory").into(),
            "--help" => {
                println!("z58-comparison [--smoke] [--output DIRECTORY]");
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }
    fs::create_dir_all(&output).unwrap();
    metadata(&output, smoke);
    let mut run = Run {
        csv: BufWriter::new(fs::File::create(output.join("raw.csv")).unwrap()),
        smoke,
    };
    writeln!(
        run.csv,
        "algorithm,operation,bytes,pattern,sample,iterations,elapsed_ns,ns_per_op"
    )
    .unwrap();
    for &bytes in LENGTHS {
        if smoke && (bytes == 8 || bytes == 16 || bytes == 128 || bytes == 4096) {
            continue;
        }
        for &pattern in PATTERNS {
            let inputs = corpus(bytes, pattern);
            let capacity = z58::encoded_capacity(bytes).unwrap();
            let mut temp = vec![0; capacity];
            let mut reference_work = vec![0; capacity];
            let expected: Vec<Vec<u8>> = inputs
                .iter()
                .map(|input| {
                    let len = reference_encode(input, &mut temp, &mut reference_work);
                    temp[..len].to_vec()
                })
                .collect();
            let mut encode_work = vec![0; z58::encode_scratch_len(bytes).unwrap()];
            let mut decode_work = vec![0; z58::decode_scratch_len(capacity).unwrap()];
            let mut byte_work = vec![0; capacity];
            let mut wide_work = vec![0; capacity];
            run.encode("z58-generic", bytes, pattern, &inputs, &expected, |i, o| {
                z58::encode(i, o, &mut encode_work).unwrap()
            });
            run.decode("z58-generic", bytes, pattern, &expected, &inputs, |i, o| {
                z58::decode(i, o, &mut decode_work).unwrap()
            });
            run.encode("bs58", bytes, pattern, &inputs, &expected, |i, o| {
                bs58::encode(i).onto(o).unwrap()
            });
            run.decode("bs58", bytes, pattern, &expected, &inputs, |i, o| {
                bs58::decode(i).onto(o).unwrap()
            });
            run.encode(
                "reference-byte",
                bytes,
                pattern,
                &inputs,
                &expected,
                |i, o| reference_encode(i, o, &mut byte_work),
            );
            run.decode(
                "reference-byte",
                bytes,
                pattern,
                &expected,
                &inputs,
                |i, o| reference_decode(i, o, &mut byte_work).unwrap(),
            );
            run.encode(
                "radix32-single",
                bytes,
                pattern,
                &inputs,
                &expected,
                |i, o| radix32_single_encode(i, o, &mut wide_work),
            );
            run.decode(
                "radix32-single",
                bytes,
                pattern,
                &expected,
                &inputs,
                |i, o| radix32_single_decode(i, o, &mut wide_work).unwrap(),
            );
            run.encode("radix32", bytes, pattern, &inputs, &expected, |i, o| {
                radix32_encode(i, o, &mut wide_work)
            });
            run.decode("radix32", bytes, pattern, &expected, &inputs, |i, o| {
                radix32_decode(i, o, &mut wide_work).unwrap()
            });
            run.encode(
                "radix32-u128",
                bytes,
                pattern,
                &inputs,
                &expected,
                |i, o| radix32_u128_encode(i, o, &mut wide_work),
            );
            run.decode(
                "radix32-u128",
                bytes,
                pattern,
                &expected,
                &inputs,
                |i, o| radix32_u128_decode(i, o, &mut wide_work).unwrap(),
            );
            run.encode("radix64", bytes, pattern, &inputs, &expected, |i, o| {
                radix64_encode(i, o, &mut wide_work)
            });
            run.decode("radix64", bytes, pattern, &expected, &inputs, |i, o| {
                radix64_decode(i, o, &mut wide_work).unwrap()
            });
            if bytes == 32 {
                run.encode(
                    "fixed-radix5-serial",
                    bytes,
                    pattern,
                    &inputs,
                    &expected,
                    |i, o| fixed_radix5_serial::encode32(i.try_into().unwrap(), o).unwrap(),
                );
                run.decode(
                    "fixed-radix5-serial",
                    bytes,
                    pattern,
                    &expected,
                    &inputs,
                    |i, o| {
                        fixed_radix5_serial::decode32(i, (&mut o[..32]).try_into().unwrap())
                            .unwrap();
                        32
                    },
                );

                run.encode(
                    "fixed-radix5-parallel",
                    bytes,
                    pattern,
                    &inputs,
                    &expected,
                    |i, o| fixed_radix5_parallel::encode32(i.try_into().unwrap(), o).unwrap(),
                );
                run.decode(
                    "fixed-radix5-parallel",
                    bytes,
                    pattern,
                    &expected,
                    &inputs,
                    |i, o| {
                        fixed_radix5_parallel::decode32(i, (&mut o[..32]).try_into().unwrap())
                            .unwrap();
                        32
                    },
                );

                run.encode(
                    "fixed-radix4-parallel",
                    bytes,
                    pattern,
                    &inputs,
                    &expected,
                    |i, o| fixed_radix4_parallel::encode32(i.try_into().unwrap(), o).unwrap(),
                );
                run.decode(
                    "fixed-radix4-parallel",
                    bytes,
                    pattern,
                    &expected,
                    &inputs,
                    |i, o| {
                        fixed_radix4_parallel::decode32(i, (&mut o[..32]).try_into().unwrap())
                            .unwrap();
                        32
                    },
                );

                run.encode("z58-fixed", bytes, pattern, &inputs, &expected, |i, o| {
                    z58::encode32(i.try_into().unwrap(), o).unwrap()
                });
                run.decode("z58-fixed", bytes, pattern, &expected, &inputs, |i, o| {
                    z58::decode32(i, (&mut o[..32]).try_into().unwrap()).unwrap();
                    32
                });
                run.encode("five8", bytes, pattern, &inputs, &expected, |i, o| {
                    five8::encode_32(i.try_into().unwrap(), (&mut o[..44]).try_into().unwrap())
                        as usize
                });
                run.decode("five8", bytes, pattern, &expected, &inputs, |i, o| {
                    five8::decode_32(i, (&mut o[..32]).try_into().unwrap()).unwrap();
                    32
                });
                run.encode("braid58", bytes, pattern, &inputs, &expected, |i, o| {
                    braid_encode32(i.try_into().unwrap(), (&mut o[..45]).try_into().unwrap())
                });
                run.decode("braid58", bytes, pattern, &expected, &inputs, |i, o| {
                    braid58::decode_32_into(i, (&mut o[..32]).try_into().unwrap()).unwrap();
                    32
                });
                run.decode("fd-bs58", bytes, pattern, &expected, &inputs, |i, o| {
                    o[..32].copy_from_slice(&fd_bs58::decode_32(i).unwrap());
                    32
                });
            } else if bytes == 64 {
                run.encode(
                    "fixed-radix5-serial",
                    bytes,
                    pattern,
                    &inputs,
                    &expected,
                    |i, o| fixed_radix5_serial::encode64(i.try_into().unwrap(), o).unwrap(),
                );
                run.decode(
                    "fixed-radix5-serial",
                    bytes,
                    pattern,
                    &expected,
                    &inputs,
                    |i, o| {
                        fixed_radix5_serial::decode64(i, (&mut o[..64]).try_into().unwrap())
                            .unwrap();
                        64
                    },
                );

                run.encode(
                    "fixed-radix5-parallel",
                    bytes,
                    pattern,
                    &inputs,
                    &expected,
                    |i, o| fixed_radix5_parallel::encode64(i.try_into().unwrap(), o).unwrap(),
                );
                run.decode(
                    "fixed-radix5-parallel",
                    bytes,
                    pattern,
                    &expected,
                    &inputs,
                    |i, o| {
                        fixed_radix5_parallel::decode64(i, (&mut o[..64]).try_into().unwrap())
                            .unwrap();
                        64
                    },
                );

                run.encode(
                    "fixed-radix4-parallel",
                    bytes,
                    pattern,
                    &inputs,
                    &expected,
                    |i, o| fixed_radix4_parallel::encode64(i.try_into().unwrap(), o).unwrap(),
                );
                run.decode(
                    "fixed-radix4-parallel",
                    bytes,
                    pattern,
                    &expected,
                    &inputs,
                    |i, o| {
                        fixed_radix4_parallel::decode64(i, (&mut o[..64]).try_into().unwrap())
                            .unwrap();
                        64
                    },
                );

                run.encode("z58-fixed", bytes, pattern, &inputs, &expected, |i, o| {
                    z58::encode64(i.try_into().unwrap(), o).unwrap()
                });
                run.decode("z58-fixed", bytes, pattern, &expected, &inputs, |i, o| {
                    z58::decode64(i, (&mut o[..64]).try_into().unwrap()).unwrap();
                    64
                });
                run.encode("five8", bytes, pattern, &inputs, &expected, |i, o| {
                    five8::encode_64(i.try_into().unwrap(), (&mut o[..88]).try_into().unwrap())
                        as usize
                });
                run.decode("five8", bytes, pattern, &expected, &inputs, |i, o| {
                    five8::decode_64(i, (&mut o[..64]).try_into().unwrap()).unwrap();
                    64
                });
                run.encode("braid58", bytes, pattern, &inputs, &expected, |i, o| {
                    braid_encode64(i.try_into().unwrap(), (&mut o[..89]).try_into().unwrap())
                });
                run.decode("braid58", bytes, pattern, &expected, &inputs, |i, o| {
                    braid58::decode_64_into(i, (&mut o[..64]).try_into().unwrap()).unwrap();
                    64
                });
                run.decode("fd-bs58", bytes, pattern, &expected, &inputs, |i, o| {
                    o[..64].copy_from_slice(&fd_bs58::decode_64(i).unwrap());
                    64
                });
            }
            run.csv.flush().unwrap();
        }
        eprintln!("verified and sampled {bytes} bytes");
    }
    println!("Raw samples: {}", output.join("raw.csv").display());
}
