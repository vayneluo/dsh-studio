use std::io;
use std::os::windows::io::AsRawHandle;
use std::process::Child;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

pub struct Job(HANDLE);

unsafe impl Send for Job {}

impl Job {
    pub fn new() -> io::Result<Self> {
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }

        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&limits).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        };
        if configured == 0 {
            let error = io::Error::last_os_error();
            unsafe { CloseHandle(handle) };
            return Err(error);
        }
        Ok(Self(handle))
    }

    pub fn assign(&self, child: &Child) -> io::Result<()> {
        let process = child.as_raw_handle() as HANDLE;
        if unsafe { AssignProcessToJobObject(self.0, process) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn assign_and_resume(&self, child: &Child) -> io::Result<()> {
        self.assign(child)?;
        resume_suspended_process(child.id())
    }
}

fn resume_suspended_process(process_id: u32) -> io::Result<()> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }

    let result = (|| {
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        let mut has_entry = unsafe { Thread32First(snapshot, &mut entry) } != 0;
        while has_entry {
            if entry.th32OwnerProcessID == process_id {
                let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
                if thread.is_null() {
                    return Err(io::Error::last_os_error());
                }
                let previous_count = unsafe { ResumeThread(thread) };
                unsafe { CloseHandle(thread) };
                if previous_count == u32::MAX {
                    return Err(io::Error::last_os_error());
                }
                if previous_count != 1 {
                    return Err(io::Error::other(format!(
                        "expected one suspended thread, found suspend count {previous_count}"
                    )));
                }
                return Ok(());
            }
            has_entry = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no thread found for suspended process {process_id}"),
        ))
    })();

    unsafe { CloseHandle(snapshot) };
    result
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::Job;
    use crate::sidecar::CREATE_NO_WINDOW;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn dropping_the_job_terminates_an_assigned_process() {
        let mut child = Command::new("cmd")
            .args(["/D", "/S", "/C", "ping -n 30 127.0.0.1 >NUL"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .unwrap();
        let job = Job::new().unwrap();
        job.assign(&child).unwrap();

        drop(job);

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                panic!("assigned process survived after the job handle closed");
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    #[test]
    fn assigned_suspended_process_is_resumed() {
        let mut child = Command::new("cmd")
            .args(["/D", "/S", "/C", "exit 0"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW | crate::sidecar::CREATE_SUSPENDED)
            .spawn()
            .unwrap();
        let job = Job::new().unwrap();

        job.assign_and_resume(&child).unwrap();

        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                panic!("assigned process remained suspended");
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    #[test]
    fn dropping_the_job_terminates_real_descendants() {
        let id = format!("{}-{:?}", std::process::id(), std::thread::current().id())
            .replace(['(', ')'], "");
        let directory = std::env::temp_dir().join(format!("dsh-studio-job-descendant-{id}"));
        std::fs::create_dir_all(&directory).unwrap();
        let started = directory.join("started.txt");
        let orphan = directory.join("orphan.txt");
        let child_script = directory.join("child.ps1");
        let root_script = directory.join("root.ps1");
        let quote = |path: &std::path::Path| path.display().to_string().replace('\'', "''");
        std::fs::write(
            &child_script,
            format!(
                "Start-Sleep -Seconds 2\nSet-Content -LiteralPath '{}' -Value orphan\n",
                quote(&orphan)
            ),
        )
        .unwrap();
        std::fs::write(
            &root_script,
            format!(
                "Start-Process -FilePath powershell.exe -WindowStyle Hidden -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '{}')\nSet-Content -LiteralPath '{}' -Value started\nStart-Sleep -Seconds 30\n",
                quote(&child_script),
                quote(&started)
            ),
        )
        .unwrap();
        let mut child = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&root_script)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW | crate::sidecar::CREATE_SUSPENDED)
            .spawn()
            .unwrap();
        let job = Job::new().unwrap();
        job.assign_and_resume(&child).unwrap();

        let started_deadline = Instant::now() + Duration::from_secs(3);
        while !started.exists() && Instant::now() < started_deadline {
            thread::sleep(Duration::from_millis(25));
        }
        assert!(started.exists(), "descendant fixture did not start");

        drop(job);
        let _ = child.wait();
        thread::sleep(Duration::from_secs(3));
        assert!(!orphan.exists(), "a descendant escaped the Job object");
        let _ = std::fs::remove_dir_all(directory);
    }
}
