/*
	Copyright 2021 Bricky <bricky149@teknik.io>

    This file is part of tlap.

    tlap is free software: you can redistribute it and/or modify
    it under the terms of the GNU Lesser General Public License as
    published by the Free Software Foundation, either version 3 of
    the License, or (at your option) any later version.

    tlap is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
    GNU Lesser General Public License for more details.

    You should have received a copy of the GNU Lesser General Public
    License along with tlap. If not, see <https://www.gnu.org/licenses/>.
*/
// This is required for when `cargo build` is run. `cargo run` needs
// environment variables set either in the helper script or in the OS.
#[cfg(target_os = "linux")]
fn main() {
	println!("cargo:rustc-env=LD_LIBRARY_PATH=~/tlap-rs/native_client/lin-cpu/");
	println!("cargo:rustc-env=LIBRARY_PATH=~/tlap-rs/native_client/lin-cpu/");
}
#[cfg(target_os = "windows")]
fn main() {
	println!(r"cargo:rustc-link-search=C:\Users\bricky\tlap-rs\native_client\win-cuda\");
	println!("cargo:rustc-link-lib=deepspeech.so.if");
}
