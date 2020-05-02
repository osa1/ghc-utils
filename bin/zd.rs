use ghc_utils::z_decode;

fn main() {
    for arg in std::env::args().skip(1) {
        match z_decode(&arg) {
            None => {
                println!();
            }
            Some(arg_z) => {
                println!("{}", arg_z);
            }
        }
    }
}
