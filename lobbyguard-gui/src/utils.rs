//! Utility functions for the LobbyGuard GUI.

/// Check if the application is running with administrator privileges.
pub fn is_running_as_admin() -> bool {
	#[cfg(windows)]
	{
		use std::mem;
		use std::ptr;

		// This is a Windows-specific check for admin privileges
		unsafe {
			let mut token_handle: *mut std::ffi::c_void = ptr::null_mut();
			let process_handle = windows_sys::Win32::System::Threading::GetCurrentProcess();

			if windows_sys::Win32::System::Threading::OpenProcessToken(
				process_handle,
				windows_sys::Win32::Security::TOKEN_QUERY,
				&mut token_handle,
			) == 0
			{
				return false;
			}

			let mut elevation: windows_sys::Win32::Foundation::BOOL = 0;
			let mut cb_size = mem::size_of::<windows_sys::Win32::Foundation::BOOL>() as u32;

			let result = windows_sys::Win32::Security::GetTokenInformation(
				token_handle,
				windows_sys::Win32::Security::TokenElevation,
				&mut elevation as *mut _ as *mut std::ffi::c_void,
				cb_size,
				&mut cb_size,
			);

			windows_sys::Win32::Foundation::CloseHandle(token_handle);

			result != 0 && elevation != 0
		}
	}
	#[cfg(not(windows))]
	{
		// On non-Windows systems, assume we have the necessary privileges
		true
	}
}
