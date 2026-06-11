#![allow(unsafe_op_in_unsafe_fn)]

use std::env;
use windows_sys::Win32::{
	Foundation::CloseHandle,
	System::{
		Memory::{
			MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_GUARD,
			PAGE_NOACCESS, PAGE_READWRITE, VirtualAlloc, VirtualFree,
		},
		Threading::{GetCurrentProcess, GetCurrentThread, OpenProcess, PROCESS_ALL_ACCESS},
	},
};
use wow64ext::*;

unsafe fn alloc_test(h_process: HANDLE) {
	const TEST_SIZE: u32 = 0x2000;
	println!("Requesting 0x{TEST_SIZE:X} bytes of memory at 0x70000020000 ...");
	let mem = VirtualAllocEx64(
		h_process,
		0x70000020000,
		TEST_SIZE,
		MEM_COMMIT | MEM_RESERVE,
		PAGE_READWRITE,
	);
	if mem == 0 {
		println!("VirtualAllocEx64 failed.");
		return;
	}
	println!("Memory allocated at: {:016X}", mem);

	let mut mbi64: MEMORY_BASIC_INFORMATION64 = std::mem::zeroed();
	let mbi64_size = std::mem::size_of::<MEMORY_BASIC_INFORMATION64>();
	VirtualQueryEx64(h_process, mem, &mut mbi64, mbi64_size as _);
	println!(
		"Query memory: {:016X} {:016X} {:08X} {:08X} {:08X}",
		mbi64.BaseAddress, mbi64.RegionSize, mbi64.Protect, mbi64.Type, mbi64.State
	);
	println!(
		"Changing protection from {:08X} to {:08X}...",
		PAGE_READWRITE, PAGE_EXECUTE_READWRITE
	);
	let mut old_protect: DWORD = 0;
	VirtualProtectEx64(
		h_process,
		mem,
		mbi64.RegionSize as _,
		PAGE_EXECUTE_READWRITE,
		&mut old_protect,
	);
	VirtualQueryEx64(h_process, mem, &mut mbi64, mbi64_size as _);
	println!(
		"Query memory: {:016X} {:016X} {:08X} {:08X} {:08X}",
		mbi64.BaseAddress, mbi64.RegionSize, mbi64.Protect, mbi64.Type, mbi64.State
	);

	print!("WriteProcessMemory64 test: ");
	let mut test_buf = vec![0u8; TEST_SIZE as usize];
	for i in 0..TEST_SIZE as usize {
		test_buf[i] = i as u8;
	}

	let mut wr_sz = 0;
	let res = WriteProcessMemory64(
		h_process,
		mem,
		test_buf.as_ptr() as *const _,
		TEST_SIZE,
		&mut wr_sz,
	);
	if res == 0 || wr_sz != TEST_SIZE {
		println!("FAILED on WriteProcessMemory64");
	} else {
		let mut cmp_buf = vec![0u8; TEST_SIZE as usize];
		let mut rd_sz = 0;
		let res = ReadProcessMemory64(
			h_process,
			mem,
			cmp_buf.as_mut_ptr() as *mut _,
			TEST_SIZE,
			&mut rd_sz,
		);
		if res == 0 || rd_sz != TEST_SIZE {
			println!("FAILED on ReadProcessMemory64");
		} else if test_buf == cmp_buf {
			println!("SUCCESS");
		} else {
			println!("FAILED on memcmp.");
		}
	}

	println!(
		"Freeing memory: {}",
		if VirtualFreeEx64(h_process, mem, 0, MEM_RELEASE) != 0 {
			"success"
		} else {
			"failure"
		}
	);
	VirtualQueryEx64(h_process, mem, &mut mbi64, mbi64_size as _);
	println!(
		"Query memory: {:016X} {:016X} {:08X} {:08X} {:08X}",
		mbi64.BaseAddress, mbi64.RegionSize, mbi64.Protect, mbi64.Type, mbi64.State
	);
}

unsafe fn run() {
	Wow64ExtInitialize();

	let wide_wow64cpu: Vec<u16> = "wow64cpu.dll\0".encode_utf16().collect();
	let turbo_dispatch: *const i8 = b"TurboDispatchJumpAddressStart\0".as_ptr() as _;
	let turbo_dispatch_addr = GetModuleHandle64(wide_wow64cpu.as_ptr());
	let turbo_dispatch = GetProcAddress64(turbo_dispatch_addr, turbo_dispatch);

	println!("tt: {turbo_dispatch:016X}");

	let args: Vec<String> = env::args().collect();
	if args.len() != 2 {
		println!("Usage:\n\t{} hex_process_ID", args[0]);
		return;
	}

	let proc_id = u32::from_str_radix(&args[1], 16).expect("Invalid process ID");
	println!("Process ID: {proc_id:08X}");

	let h_process = OpenProcess(PROCESS_ALL_ACCESS, 0, proc_id);
	if h_process == 0 {
		panic!("Can't open process {proc_id:08X}.");
	}

	let mut mbi64: MEMORY_BASIC_INFORMATION64 = std::mem::zeroed();
	let mbi64_size = std::mem::size_of::<MEMORY_BASIC_INFORMATION64>();
	let mut cr_addr: DWORD64 = 0;
	let print_mem_map = true;

	while VirtualQueryEx64(h_process as _, cr_addr, &mut mbi64, mbi64_size as _) != 0 {
		if mbi64.Protect != 0 && (mbi64.Protect & (PAGE_NOACCESS | PAGE_GUARD)) == 0 {
			if print_mem_map {
				print!("[D] : ");
			}

			let mem = VirtualAlloc(0 as _, mbi64.RegionSize as _, MEM_COMMIT, PAGE_READWRITE);
			if mem.is_null() {
				println!("VirtualAlloc failed");
				cr_addr = cr_addr + mbi64.RegionSize;
				continue;
			}

			let mut rd_pm = 0;
			let res = ReadProcessMemory64(
				h_process as _,
				mbi64.BaseAddress,
				mem,
				mbi64.RegionSize as _,
				&mut rd_pm,
			);
			if res == 0 || rd_pm != mbi64.RegionSize as _ {
				if print_mem_map {
					print!(
						"{:016X} : {:016X} : {:08X} : ",
						mbi64.BaseAddress, mbi64.RegionSize, mbi64.Protect
					);
				}
				println!("ReadProcessMemory failed");
				VirtualFree(mem, 0, MEM_RELEASE);
				cr_addr = cr_addr + mbi64.RegionSize;
				continue;
			}

			let file_name = format!(
				"{:08X}_{:016X}_{:08X}.bin",
				proc_id, mbi64.BaseAddress, mbi64.Protect
			);
			let bytes = std::slice::from_raw_parts(mem as *const u8, mbi64.RegionSize as usize);
			std::fs::write(file_name, bytes).expect("WriteFile failed");

			VirtualFree(mem, 0, MEM_RELEASE);
		} else {
			if print_mem_map {
				print!("[ ] : ");
			}
		}

		if print_mem_map {
			println!(
				"{:016X} : {:016X} : {:08X}",
				mbi64.BaseAddress, mbi64.RegionSize, mbi64.Protect
			);
		}

		cr_addr = cr_addr + mbi64.RegionSize;
	}

	let wide_ntdll: Vec<u16> = "ntdll.dll\0".encode_utf16().collect();
	let ntdll64 = GetModuleHandle64(wide_ntdll.as_ptr());
	println!("\nNTDLL64: {ntdll64:016X}\n");

	let rtlcrc32 = GetProcAddress64(ntdll64, b"RtlComputeCrc32\0".as_ptr() as *const _);
	println!("RtlComputeCrc32 address: {rtlcrc32:016X}");

	if rtlcrc32 != 0 {
		let ret = X64Call(rtlcrc32, 3, 0u64, "ReWolf\0".as_ptr(), 6u32);
		println!("CRC32(\"ReWolf\") = {ret:016X}\n");
	}

	println!("Alloc/Protect/Write/Free test:");
	alloc_test(h_process as _);

	println!("\nAlloc/Protect/Write/Free over 4GB inside WoW64 test:");
	alloc_test(GetCurrentProcess() as _);

	println!("\n\nGet/Set Context test:");

	let mut ctx: _CONTEXT64 = std::mem::zeroed();
	ctx.ContextFlags = CONTEXT64_ALL;
	GetThreadContext64(GetCurrentThread() as _, &mut ctx);

	println!("rsp: {:016X}", ctx.Rsp);
	println!("rip: {:016X}", ctx.Rip);
	println!("r8 : {:016X}", ctx.R8);
	println!("r9 : {:016X}", ctx.R9);
	println!("r12: {:016X}", ctx.R12);

	CloseHandle(h_process);
}

fn main() {
	unsafe { run() }
}
