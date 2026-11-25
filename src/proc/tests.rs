//! Pins the container itself rather than the symptoms it prevents: the deadline ends a job object in
//! one kernel call, and closing the container is not a kill (#384).

use super::{Ended, shell, spawn_group, supervise};
use std::process::Command;
use std::time::{Duration, Instant};

/// The container is a job object on Windows, and a child that outlives its budget ends as a timeout
/// over that job rather than as a `taskkill /F /T` walk. A refused job would put the walk back, so the
/// probe asserts the job exists before it asserts anything about the deadline.
#[cfg(windows)]
#[test]
fn the_tree_kill_is_one_act_over_a_job_object_not_a_walk() {
    let (shell, flag) = shell();
    let mut cmd = Command::new(shell);
    cmd.args([flag, "ping -n 30 127.0.0.1 >NUL"]);
    let mut grouped = spawn_group(&mut cmd).expect("the probe's own child must start");

    assert!(
        grouped.job.is_some(),
        "Windows refused the job object: the deadline is back on the `taskkill /F /T` walk"
    );

    let ended = supervise(&mut grouped, Instant::now(), Some(Duration::from_millis(200)))
        .expect("a supervised wait must not fail on this host");
    assert!(
        matches!(ended, Ended::TimedOut { .. }),
        "a child that outlives the budget must end as a timeout, not as an exit"
    );
}

/// The job is created without `KILL_ON_JOB_CLOSE`, so closing it must not kill a descendant the
/// child deliberately left running: that is what `run_capture` documents as preserved.
#[test]
fn closing_the_container_does_not_kill_what_the_child_left_running() {
    let dir = std::env::temp_dir();
    let name = format!("cutver-container-descendant-{}.marker", std::process::id());
    let marker = dir.join(&name);
    let _ = std::fs::remove_file(&marker);
    // A bare name, with the child started in `dir`: a path that carries a space cannot be quoted
    // inside a `cmd /C` command line, because `Command` quotes by C rules that `cmd` does not read.
    let script = if cfg!(windows) {
        // The carets keep `&` and `>` for the descendant's own command line.
        format!("start /b cmd /c ping -n 3 127.0.0.1 >NUL ^& type nul ^> {name}")
    } else {
        format!("(sleep 2; touch {name}) &")
    };

    let (shell, flag) = shell();
    let mut cmd = Command::new(shell);
    cmd.args([flag, script.as_str()]).current_dir(&dir);
    let mut grouped = spawn_group(&mut cmd).expect("the probe's own child must start");

    let ended = supervise(&mut grouped, Instant::now(), Some(Duration::from_secs(10)))
        .expect("a supervised wait must not fail on this host");
    assert!(
        matches!(ended, Ended::Exited(_)),
        "the direct child exits at once, well inside its budget"
    );

    drop(grouped);
    std::thread::sleep(Duration::from_secs(4));
    let survived = marker.exists();
    let _ = std::fs::remove_file(&marker);
    assert!(
        survived,
        "closing the container killed a descendant the child left behind: the job must not carry \
         `KILL_ON_JOB_CLOSE`"
    );
}
