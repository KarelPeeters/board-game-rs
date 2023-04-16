use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoroshiro64StarStar;

fn main() {
    // tell cargo it doesn't need to rerun every time
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    generate_go_files(out_dir);
}

const GO_MAX_SIZE: u8 = u8::MAX - 2;
const GO_MAX_AREA: u16 = GO_MAX_SIZE as u16 * GO_MAX_SIZE as u16;

type GoHashInner = u128;

const GO_HASH_SIZE_BYTES: u32 = GoHashInner::BITS / 8;
const GO_HASH_COUNT: u32 = 2 * (GO_MAX_AREA as u32);

fn generate_go_files(out_dir: &Path) {
    let path_const = out_dir.join("go_consts.rs");
    let path_hash_code = out_dir.join("go_hash_code.rs");
    let path_hash_data = out_dir.join("go_hash_data.bin");

    // write constants
    {
        let mut writer_const = BufWriter::new(File::create(path_const).unwrap());
        let f = &mut writer_const;
        writeln!(f, "pub const GO_MAX_SIZE: u8 = {};", GO_MAX_SIZE).unwrap();
        writeln!(f, "pub const GO_MAX_AREA: u16 = {};", GO_MAX_AREA).unwrap();
        f.flush().unwrap();
    }

    // write random hash data
    {
        let mut rng = consistent_rng();

        // write small arrays as rust cost
        let mut writer_code = BufWriter::new(File::create(path_hash_code).unwrap());
        let f = &mut writer_code;
        writeln!(f, "type Inner = u{};", GoHashInner::BITS).unwrap();
        writeln!(f, "const GO_HASH_COUNT: usize = {};", GO_HASH_COUNT).unwrap();
        write_const_array(f, "HASH_DATA_TURN", 2, &mut rng).unwrap();
        write_const_array(f, "HASH_DATA_PASS", 3, &mut rng).unwrap();
        f.flush().unwrap();

        // write large array as binary data
        let mut writer_data = BufWriter::new(File::create(path_hash_data).unwrap());
        let go_hash_bytes = GO_HASH_COUNT as usize * GO_HASH_SIZE_BYTES as usize;
        write_random_bytes(&mut writer_data, go_hash_bytes, &mut rng).unwrap();
        writer_data.flush().unwrap();
    }
}

fn write_const_array(f: &mut impl Write, name: &str, len: usize, rng: &mut impl Rng) -> std::io::Result<()> {
    write!(f, "const {}: [u{}; {}] = ", name, GoHashInner::BITS, len)?;

    write!(f, "[")?;
    for i in 0..len {
        if i != 0 {
            write!(f, ", ")?;
        }
        let value = rng.gen::<GoHashInner>();
        write!(f, "0x{:0width$x}", value, width = (GO_HASH_SIZE_BYTES * 2) as usize)?;
    }
    write!(f, "];")?;

    Ok(())
}

fn write_random_bytes(mut writer: impl Write, bytes: usize, rng: &mut impl Rng) -> std::io::Result<()> {
    // generate large chunks of random data so we don't waste any and need excessive rng iterations
    type Chunk = u64;
    const CHUNK_BYTES: usize = (Chunk::BITS / 8) as usize;

    let mut bytes_left = bytes;
    while bytes_left > CHUNK_BYTES {
        writer.write_all(&rng.gen::<Chunk>().to_ne_bytes())?;
        bytes_left -= CHUNK_BYTES;
    }
    while bytes_left > 0 {
        writer.write_all(&rng.gen::<u8>().to_ne_bytes())?;
        bytes_left -= 1;
    }
    Ok(())
}

fn consistent_rng() -> impl Rng {
    Xoroshiro64StarStar::seed_from_u64(0)
}
