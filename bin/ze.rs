use ghc_utils::z_encode;

fn main() {
    for arg in std::env::args().skip(1) {
        match z_encode(&arg) {
            None => {
                println!("");
            }
            Some(arg_z) => {
                println!("{}", arg_z);
            }
        }
    }
}
