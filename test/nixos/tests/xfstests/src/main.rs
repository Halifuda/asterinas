// SPDX-License-Identifier: MPL-2.0

//! The test suite for exFAT refactor filesystem validation on Asterinas NixOS.

use std::env;

use nixos_test_framework::*;

nixos_test_main!();

const GUEST_FILE_CHUNK_LEN: usize = 12;

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[nixos_test]
fn exfat_refactor_boot_prompt_probe(_nixos_shell: &mut Session) -> Result<(), Error> {
    Ok(())
}

fn write_guest_file(nixos_shell: &mut Session, path: &str, lines: &[&str]) -> Result<(), Error> {
    nixos_shell.run_cmd(&format!("rm -f {}", path))?;

    for (index, line) in lines.iter().enumerate() {
        let redirect = if index == 0 { ">" } else { ">>" };

        if line.is_empty() {
            nixos_shell.run_cmd(&format!("echo {}{}", redirect, path))?;
            continue;
        }

        for (chunk_index, chunk) in line.as_bytes().chunks(GUEST_FILE_CHUNK_LEN).enumerate() {
            let chunk = std::str::from_utf8(chunk).expect("guest file chunks stay UTF-8");
            let redirect = if chunk_index == 0 { redirect } else { ">>" };
            nixos_shell.run_cmd(&format!(
                "printf %s {}{}{}",
                shell_single_quote(chunk),
                redirect,
                path
            ))?;
        }

        nixos_shell.run_cmd(&format!("echo >>{}", path))?;
    }

    Ok(())
}

#[nixos_test]
fn exfat_refactor_mount_smoke(nixos_shell: &mut Session) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());
    let scratch_dev = env::var("XFSTESTS_SCRATCH_DEV").unwrap_or_else(|_| "/dev/vdc".to_string());

    nixos_shell.run_cmd_and_expect("cat /proc/filesystems", "exfat_refactor")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo sd", scratch_dev), "sd")?;

    nixos_shell.run_cmd("mkdir -p /t /s")?;

    nixos_shell.run_cmd_and_expect(
        &format!("mount -t exfat_refactor {} /t&&echo mt", test_dev),
        "mt",
    )?;
    nixos_shell.run_cmd_and_expect(
        &format!("mount -t exfat_refactor {} /s&&echo ms", scratch_dev),
        "ms",
    )?;
    nixos_shell.run_cmd_and_expect("printf x >/t/f&&echo wr", "wr")?;
    nixos_shell.run_cmd_and_expect("sync&&echo sy", "sy")?;
    nixos_shell.run_cmd_and_expect("umount /s&&echo us", "us")?;
    nixos_shell.run_cmd_and_expect("umount /t&&echo ut", "ut")?;
    nixos_shell.run_cmd_and_expect(
        &format!("mount -t exfat_refactor {} /t&&echo rt", test_dev),
        "rt",
    )?;
    nixos_shell.run_cmd_and_expect("cat /t/f", "x")?;
    nixos_shell.run_cmd_and_expect("umount /t&&echo cu", "cu")?;

    Ok(())
}

#[nixos_test]
fn exfat_refactor_s1a_named_batch(nixos_shell: &mut Session) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect("cat /proc/filesystems", "exfat_refactor")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;

    write_guest_file(
        nixos_shell,
        "m",
        &[
            "#!/bin/sh",
            "log=/tmp/b/mkfs-wrapper.log",
            "log_line() {",
            "line=\"$(date +%s) $*\"",
            "printf '%s\\n' \"$line\" >>\"$log\"",
            "printf '%s\\n' \"$line\" >/dev/console 2>/dev/null || true",
            "}",
            "log_line mkfs.exfat start \"$*\"",
            "/nix/var/nix/profiles/system/sw/bin/mkfs.exfat \"$@\"",
            "rc=$?",
            "log_line mkfs.exfat end rc=$rc",
            "exit \"$rc\"",
        ],
    )?;
    write_guest_file(
        nixos_shell,
        "f",
        &[
            "#!/bin/sh",
            "log=/tmp/b/fsck-wrapper.log",
            "log_line() {",
            "line=\"$(date +%s) $*\"",
            "printf '%s\\n' \"$line\" >>\"$log\"",
            "printf '%s\\n' \"$line\" >/dev/console 2>/dev/null || true",
            "}",
            "log_line fsck.exfat start \"$*\"",
            "/nix/var/nix/profiles/system/sw/bin/fsck.exfat \"$@\"",
            "rc=$?",
            "log_line fsck.exfat end rc=$rc",
            "exit \"$rc\"",
        ],
    )?;
    write_guest_file(
        nixos_shell,
        "mt",
        &[
            "#!/bin/sh",
            "log=/tmp/b/mount-wrapper.log",
            "log_line() {",
            "line=\"$(date +%s) $*\"",
            "printf '%s\\n' \"$line\" >>\"$log\"",
            "printf '%s\\n' \"$line\" >/dev/console 2>/dev/null || true",
            "}",
            "log_line mount start \"$*\"",
            "/nix/var/nix/profiles/system/sw/bin/mount \"$@\"",
            "rc=$?",
            "log_line mount end rc=$rc",
            "exit \"$rc\"",
        ],
    )?;
    write_guest_file(
        nixos_shell,
        "ut",
        &[
            "#!/bin/sh",
            "log=/tmp/b/umount-wrapper.log",
            "log_line() {",
            "line=\"$(date +%s) $*\"",
            "printf '%s\\n' \"$line\" >>\"$log\"",
            "printf '%s\\n' \"$line\" >/dev/console 2>/dev/null || true",
            "}",
            "log_line umount start \"$*\"",
            "/nix/var/nix/profiles/system/sw/bin/umount \"$@\"",
            "rc=$?",
            "log_line umount end rc=$rc",
            "exit \"$rc\"",
        ],
    )?;
    write_guest_file(
        nixos_shell,
        "ft",
        &[
            "#!/bin/sh",
            "log=/tmp/b/findmnt-wrapper.log",
            "log_line() {",
            "line=\"$(date +%s) $*\"",
            "printf '%s\\n' \"$line\" >>\"$log\"",
            "printf '%s\\n' \"$line\" >/dev/console 2>/dev/null || true",
            "}",
            "log_line findmnt start \"$*\"",
            "/nix/var/nix/profiles/system/sw/bin/findmnt \"$@\"",
            "rc=$?",
            "log_line findmnt end rc=$rc",
            "exit \"$rc\"",
        ],
    )?;
    write_guest_file(
        nixos_shell,
        "sy",
        &[
            "#!/bin/sh",
            "log=/tmp/b/sync-wrapper.log",
            "log_line() {",
            "line=\"$(date +%s) $*\"",
            "printf '%s\\n' \"$line\" >>\"$log\"",
            "printf '%s\\n' \"$line\" >/dev/console 2>/dev/null || true",
            "}",
            "log_line sync start \"$*\"",
            "/nix/var/nix/profiles/system/sw/bin/sync \"$@\"",
            "rc=$?",
            "log_line sync end rc=$rc",
            "exit \"$rc\"",
        ],
    )?;
    write_guest_file(
        nixos_shell,
        "l",
        &[
            "export FSTYP=exfat_refactor",
            "export TEST_DEV=/dev/vdb",
            "export TEST_DIR=/mnt/test",
            "export SCRATCH_MNT=/mnt/scratch",
            "export RESULT_BASE=/tmp/o",
        ],
    )?;
    write_guest_file(
        nixos_shell,
        "r",
        &[
            "#!/bin/sh",
            "set -eu",
            "export PATH=/nix/var/nix/profiles/system/sw/bin:$PATH",
            "mkdir -p /tmp/x /tmp/b /tmp/o /mnt/test /mnt/scratch",
            "log_marker() {",
            "line=\"$(date +%s) $*\"",
            "printf '%s\\n' \"$line\" >>/tmp/b/progress.log",
            "printf '%s\\n' \"$line\" >/dev/console 2>/dev/null || true",
            "}",
            "heartbeat() {",
            "check_pid=$1",
            "while kill -0 \"$check_pid\" 2>/dev/null; do",
            "bytes=0",
            "if [ -f /tmp/b/check.stdout ]; then",
            "bytes=$(wc -c </tmp/b/check.stdout 2>/dev/null || echo 0)",
            "fi",
            "log_marker heartbeat pid=$check_pid check_stdout_bytes=$bytes",
            "ps -o pid,stat,etimes,args -p \"$check_pid\" >>/tmp/b/check-ps.log 2>/dev/null || true",
            "sleep 10",
            "done",
            "}",
            "emit_export() {",
            "echo XFSTESTS_BATCH_EXPORT_BEGIN",
            "for file in /tmp/b/* /tmp/b/*/* /tmp/o/* /tmp/o/*/*; do",
            "[ -f \"$file\" ] || continue",
            "echo XFSTESTS_FILE_BEGIN:\"$file\"",
            "base64 \"$file\"",
            "echo XFSTESTS_FILE_END:\"$file\"",
            "done",
            "echo XFSTESTS_BATCH_EXPORT_END",
            "}",
            "trap 'st=$?; echo \"$st\" >/tmp/b/check.exit_status; emit_export; trap - EXIT; exit 0' EXIT",
            "cp /tmp/m /tmp/x/mkfs.exfat_refactor",
            "cp /tmp/f /tmp/x/fsck.exfat_refactor",
            "cp /tmp/mt /tmp/x/mount",
            "cp /tmp/ut /tmp/x/umount",
            "cp /tmp/ft /tmp/x/findmnt",
            "cp /tmp/sy /tmp/x/sync",
            "chmod +x /tmp/x/mkfs.exfat_refactor /tmp/x/fsck.exfat_refactor /tmp/x/mount /tmp/x/umount /tmp/x/findmnt /tmp/x/sync",
            "check_path=/nix/var/nix/profiles/system/sw/bin/xfstests-check",
            "echo \"$PATH\" >/tmp/b/path.txt",
            "ls -l \"$check_path\" >/tmp/b/check-path-ls.txt 2>&1 || true",
            "test -x \"$check_path\"",
            "check_path=$(realpath \"$check_path\")",
            "suite_root=$(dirname \"$check_path\")",
            "if [ -e \"$suite_root/common/preamble\" ]; then",
            ":",
            "elif [ -e \"$(dirname \"$suite_root\")/share/xfstests/common/preamble\" ]; then",
            "suite_root=$(dirname \"$suite_root\")/share/xfstests",
            "elif [ -e \"$(dirname \"$suite_root\")/xfstests-dev/common/preamble\" ]; then",
            "suite_root=$(dirname \"$suite_root\")/xfstests-dev",
            "fi",
            "echo \"$check_path\" >/tmp/b/check-path.txt",
            "echo \"$suite_root\" >/tmp/b/suite-root.txt",
            "echo 57d71a884dd1b3b3c44a27d2d106b3be84ddc5fb >/tmp/b/source-revision.txt",
            "printf '%s\\n' generic/001 generic/007 >/tmp/b/selected-tests.txt",
            "printf '%s\\n' generic/013 >>/tmp/b/selected-tests.txt",
            "umount /mnt/scratch 2>/dev/null || true",
            "umount /mnt/test 2>/dev/null || true",
            "cp /tmp/l /tmp/b/local.config",
            "cp /tmp/r /tmp/b/run.sh",
            "set +e",
            "log_marker before-xfstests-check",
            "HOST_OPTIONS=/tmp/l \\",
            "RESULT_BASE=/tmp/o \\",
            "PATH=/tmp/x:$PATH \\",
            "\"$check_path\" \\",
            "generic/001 \\",
            "generic/007 \\",
            "generic/013 \\",
            ">/tmp/b/check.stdout 2>&1 &",
            "check_pid=$!",
            "echo \"$check_pid\" >/tmp/b/check.pid",
            "log_marker after-spawn-xfstests-check pid=$check_pid",
            "tail -n +1 -f /tmp/b/check.stdout >/dev/console 2>/dev/null &",
            "tail_pid=$!",
            "log_marker after-spawn-tail pid=$tail_pid",
            "heartbeat \"$check_pid\" &",
            "heartbeat_pid=$!",
            "wait \"$check_pid\"",
            "st=$?",
            "kill \"$tail_pid\" 2>/dev/null || true",
            "wait \"$tail_pid\" 2>/dev/null || true",
            "kill \"$heartbeat_pid\" 2>/dev/null || true",
            "wait \"$heartbeat_pid\" 2>/dev/null || true",
            "log_marker after-wait-xfstests-check rc=$st",
            "set -e",
            "cp /tmp/b/check.stdout /tmp/o/batch-stdout.txt",
            "findmnt -rncv -S /dev/vdb -o SOURCE,FSTYPE,TARGET >/tmp/b/mounted-fs.txt 2>/dev/null || true",
            "echo \"$st\" >/tmp/b/check.exit_status",
            "trap - EXIT",
            "emit_export",
            "exit 0",
        ],
    )?;

    nixos_shell.run_cmd("chmod +x /tmp/r")?;
    nixos_shell.run_cmd_and_expect("/tmp/r", "XFSTESTS_BATCH_EXPORT_END")?;

    Ok(())
}

#[nixos_test]
fn exfat_refactor_fsck_same_image_probe(nixos_shell: &mut Session) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect("cat /proc/filesystems", "exfat_refactor")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;
    write_guest_file(
        nixos_shell,
        "p",
        &[
            "#!/bin/sh",
            "set +e",
            "test_dev=${XFSTESTS_TEST_DEV:-/dev/vdb}",
            "export PATH=/nix/var/nix/profiles/system/sw/bin:$PATH",
            "helper=/nix/var/nix/profiles/system/sw/bin/fsck.exfat",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/fsck-probe-mounted-before.txt 2>&1 || true",
            "command -v fsck.exfat >/tmp/fsck-probe-helper-path.txt 2>&1 || true",
            "if [ -x \"$helper\" ]; then",
            "\"$helper\" -n \"$test_dev\" >/tmp/fsck-probe.out 2>&1",
            "rc=$?",
            "else",
            "printf 'missing exFAT checker: %s\\n' \"$helper\" >/tmp/fsck-probe.out",
            "rc=127",
            "fi",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/fsck-probe-mounted-after.txt 2>&1 || true",
            "echo FSCK_PROBE_HELPER_ABS=$helper",
            "echo FSCK_PROBE_HELPER_BEGIN",
            "cat /tmp/fsck-probe-helper-path.txt",
            "echo FSCK_PROBE_HELPER_END",
            "echo FSCK_PROBE_MOUNTED_BEFORE_BEGIN",
            "cat /tmp/fsck-probe-mounted-before.txt",
            "echo FSCK_PROBE_MOUNTED_BEFORE_END",
            "echo FSCK_PROBE_RC=$rc",
            "echo FSCK_PROBE_OUTPUT_BEGIN",
            "cat /tmp/fsck-probe.out",
            "echo FSCK_PROBE_OUTPUT_END",
            "echo FSCK_PROBE_MOUNTED_AFTER_BEGIN",
            "cat /tmp/fsck-probe-mounted-after.txt",
            "echo FSCK_PROBE_MOUNTED_AFTER_END",
            "echo FSCK_PROBE_DONE",
            "exit 0",
        ],
    )?;
    nixos_shell.run_cmd("chmod +x /tmp/p")?;
    nixos_shell.run_cmd_and_expect(
        &format!("XFSTESTS_TEST_DEV={} /tmp/p", test_dev),
        "FSCK_PROBE_DONE",
    )?;

    Ok(())
}

#[nixos_test]
fn exfat_refactor_fsck_same_image_verbose_probe(
    nixos_shell: &mut Session,
) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect("cat /proc/filesystems", "exfat_refactor")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;
    write_guest_file(
        nixos_shell,
        "pfv",
        &[
            "#!/bin/sh",
            "set +e",
            "test_dev=${XFSTESTS_TEST_DEV:-/dev/vdb}",
            "export PATH=/nix/var/nix/profiles/system/sw/bin:$PATH",
            "helper=/nix/var/nix/profiles/system/sw/bin/fsck.exfat",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/fsck-verbose-probe-mounted-before.txt 2>&1 || true",
            "command -v fsck.exfat >/tmp/fsck-verbose-probe-helper-path.txt 2>&1 || true",
            "if [ -x \"$helper\" ]; then",
            "\"$helper\" -n -vv \"$test_dev\" >/tmp/fsck-verbose-probe.out 2>&1",
            "rc=$?",
            "else",
            "printf 'missing exFAT checker: %s\\n' \"$helper\" >/tmp/fsck-verbose-probe.out",
            "rc=127",
            "fi",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/fsck-verbose-probe-mounted-after.txt 2>&1 || true",
            "echo FSCK_VERBOSE_PROBE_HELPER_ABS=$helper",
            "echo FSCK_VERBOSE_PROBE_HELPER_BEGIN",
            "cat /tmp/fsck-verbose-probe-helper-path.txt",
            "echo FSCK_VERBOSE_PROBE_HELPER_END",
            "echo FSCK_VERBOSE_PROBE_MOUNTED_BEFORE_BEGIN",
            "cat /tmp/fsck-verbose-probe-mounted-before.txt",
            "echo FSCK_VERBOSE_PROBE_MOUNTED_BEFORE_END",
            "echo FSCK_VERBOSE_PROBE_RC=$rc",
            "echo FSCK_VERBOSE_PROBE_OUTPUT_BEGIN",
            "cat /tmp/fsck-verbose-probe.out",
            "echo FSCK_VERBOSE_PROBE_OUTPUT_END",
            "echo FSCK_VERBOSE_PROBE_MOUNTED_AFTER_BEGIN",
            "cat /tmp/fsck-verbose-probe-mounted-after.txt",
            "echo FSCK_VERBOSE_PROBE_MOUNTED_AFTER_END",
            "echo FSCK_VERBOSE_PROBE_DONE",
            "exit 0",
        ],
    )?;
    nixos_shell.run_cmd("chmod +x /tmp/pfv")?;
    nixos_shell.run_cmd_and_expect(
        &format!("XFSTESTS_TEST_DEV={} /tmp/pfv", test_dev),
        "FSCK_VERBOSE_PROBE_DONE",
    )?;

    Ok(())
}

#[nixos_test]
fn ext2_blockdev_fsck_probe(nixos_shell: &mut Session) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;
    write_guest_file(
        nixos_shell,
        "pe2",
        &[
            "#!/bin/sh",
            "set +e",
            "test_dev=${XFSTESTS_TEST_DEV:-/dev/vdb}",
            "export PATH=/nix/var/nix/profiles/system/sw/bin:$PATH",
            "mkfs_helper=/nix/var/nix/profiles/system/sw/bin/mkfs.ext2",
            "fsck_helper=/nix/var/nix/profiles/system/sw/bin/e2fsck",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/ext2-probe-mounted-before.txt 2>&1 || true",
            "umount \"$test_dev\" 2>/dev/null || true",
            "command -v mkfs.ext2 >/tmp/ext2-probe-mkfs-helper-path.txt 2>&1 || true",
            "command -v e2fsck >/tmp/ext2-probe-fsck-helper-path.txt 2>&1 || true",
            "if [ -x \"$mkfs_helper\" ]; then",
            "\"$mkfs_helper\" -F \"$test_dev\" >/tmp/ext2-probe-mkfs.out 2>&1",
            "mkfs_rc=$?",
            "else",
            "printf 'missing ext2 mkfs helper: %s\\n' \"$mkfs_helper\" >/tmp/ext2-probe-mkfs.out",
            "mkfs_rc=127",
            "fi",
            "if [ $mkfs_rc -eq 0 ] && [ -x \"$fsck_helper\" ]; then",
            "\"$fsck_helper\" -f -n \"$test_dev\" >/tmp/ext2-probe-fsck.out 2>&1",
            "fsck_rc=$?",
            "elif [ ! -x \"$fsck_helper\" ]; then",
            "printf 'missing ext2 fsck helper: %s\\n' \"$fsck_helper\" >/tmp/ext2-probe-fsck.out",
            "fsck_rc=127",
            "else",
            "printf 'skipped e2fsck because mkfs.ext2 failed rc=%s\\n' \"$mkfs_rc\" >/tmp/ext2-probe-fsck.out",
            "fsck_rc=125",
            "fi",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/ext2-probe-mounted-after.txt 2>&1 || true",
            "echo EXT2_PROBE_MKFS_HELPER_ABS=$mkfs_helper",
            "echo EXT2_PROBE_MKFS_HELPER_BEGIN",
            "cat /tmp/ext2-probe-mkfs-helper-path.txt",
            "echo EXT2_PROBE_MKFS_HELPER_END",
            "echo EXT2_PROBE_FSCK_HELPER_ABS=$fsck_helper",
            "echo EXT2_PROBE_FSCK_HELPER_BEGIN",
            "cat /tmp/ext2-probe-fsck-helper-path.txt",
            "echo EXT2_PROBE_FSCK_HELPER_END",
            "echo EXT2_PROBE_MOUNTED_BEFORE_BEGIN",
            "cat /tmp/ext2-probe-mounted-before.txt",
            "echo EXT2_PROBE_MOUNTED_BEFORE_END",
            "echo EXT2_PROBE_MKFS_RC=$mkfs_rc",
            "echo EXT2_PROBE_MKFS_OUTPUT_BEGIN",
            "cat /tmp/ext2-probe-mkfs.out",
            "echo EXT2_PROBE_MKFS_OUTPUT_END",
            "echo EXT2_PROBE_FSCK_RC=$fsck_rc",
            "echo EXT2_PROBE_FSCK_OUTPUT_BEGIN",
            "cat /tmp/ext2-probe-fsck.out",
            "echo EXT2_PROBE_FSCK_OUTPUT_END",
            "echo EXT2_PROBE_MOUNTED_AFTER_BEGIN",
            "cat /tmp/ext2-probe-mounted-after.txt",
            "echo EXT2_PROBE_MOUNTED_AFTER_END",
            "echo EXT2_PROBE_DONE",
            "exit 0",
        ],
    )?;
    nixos_shell.run_cmd("chmod +x /tmp/pe2")?;
    nixos_shell.run_cmd_and_expect(
        &format!("XFSTESTS_TEST_DEV={} /tmp/pe2", test_dev),
        "EXT2_PROBE_DONE",
    )?;

    Ok(())
}

#[nixos_test]
fn ext2_blockdev_fsck_strace_probe(nixos_shell: &mut Session) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;
    write_guest_file(
        nixos_shell,
        "pe2s",
        &[
            "#!/bin/sh",
            "set +e",
            "test_dev=${XFSTESTS_TEST_DEV:-/dev/vdb}",
            "export PATH=/nix/var/nix/profiles/system/sw/bin:$PATH",
            "mkfs_helper=/nix/var/nix/profiles/system/sw/bin/mkfs.ext2",
            "fsck_helper=/nix/var/nix/profiles/system/sw/bin/e2fsck",
            "strace_helper=/nix/var/nix/profiles/system/sw/bin/strace",
            "trace_file=/tmp/ext2-probe-fsck.strace",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/ext2-strace-probe-mounted-before.txt 2>&1 || true",
            "umount \"$test_dev\" 2>/dev/null || true",
            "if [ -x \"$mkfs_helper\" ]; then",
            "\"$mkfs_helper\" -F \"$test_dev\" >/tmp/ext2-strace-probe-mkfs.out 2>&1",
            "mkfs_rc=$?",
            "else",
            "printf 'missing ext2 mkfs helper: %s\\n' \"$mkfs_helper\" >/tmp/ext2-strace-probe-mkfs.out",
            "mkfs_rc=127",
            "fi",
            "if [ $mkfs_rc -eq 0 ] && [ -x \"$fsck_helper\" ] && [ -x \"$strace_helper\" ]; then",
            "\"$strace_helper\" -o \"$trace_file\" -yy -s 128 -e trace=pread64,read,readv,preadv,preadv2,lseek,ioctl \"$fsck_helper\" -f -n \"$test_dev\" >/tmp/ext2-strace-probe-fsck.out 2>&1",
            "fsck_rc=$?",
            "elif [ ! -x \"$fsck_helper\" ]; then",
            "printf 'missing ext2 fsck helper: %s\\n' \"$fsck_helper\" >/tmp/ext2-strace-probe-fsck.out",
            "fsck_rc=127",
            "elif [ ! -x \"$strace_helper\" ]; then",
            "printf 'missing strace helper: %s\\n' \"$strace_helper\" >/tmp/ext2-strace-probe-fsck.out",
            "fsck_rc=126",
            "else",
            "printf 'skipped e2fsck because mkfs.ext2 failed rc=%s\\n' \"$mkfs_rc\" >/tmp/ext2-strace-probe-fsck.out",
            "fsck_rc=125",
            "fi",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/ext2-strace-probe-mounted-after.txt 2>&1 || true",
            "echo EXT2_STRACE_PROBE_MKFS_RC=$mkfs_rc",
            "echo EXT2_STRACE_PROBE_MKFS_OUTPUT_BEGIN",
            "cat /tmp/ext2-strace-probe-mkfs.out",
            "echo EXT2_STRACE_PROBE_MKFS_OUTPUT_END",
            "echo EXT2_STRACE_PROBE_FSCK_RC=$fsck_rc",
            "echo EXT2_STRACE_PROBE_FSCK_OUTPUT_BEGIN",
            "cat /tmp/ext2-strace-probe-fsck.out",
            "echo EXT2_STRACE_PROBE_FSCK_OUTPUT_END",
            "echo EXT2_STRACE_PROBE_TRACE_BEGIN",
            "cat \"$trace_file\" 2>/dev/null || true",
            "echo EXT2_STRACE_PROBE_TRACE_END",
            "echo EXT2_STRACE_PROBE_MOUNTED_BEFORE_BEGIN",
            "cat /tmp/ext2-strace-probe-mounted-before.txt",
            "echo EXT2_STRACE_PROBE_MOUNTED_BEFORE_END",
            "echo EXT2_STRACE_PROBE_MOUNTED_AFTER_BEGIN",
            "cat /tmp/ext2-strace-probe-mounted-after.txt",
            "echo EXT2_STRACE_PROBE_MOUNTED_AFTER_END",
            "echo EXT2_STRACE_PROBE_DONE",
            "exit 0",
        ],
    )?;
    nixos_shell.run_cmd("chmod +x /tmp/pe2s")?;
    nixos_shell.run_cmd_and_expect(
        &format!("XFSTESTS_TEST_DEV={} /tmp/pe2s", test_dev),
        "EXT2_STRACE_PROBE_DONE",
    )?;

    Ok(())
}

#[nixos_test]
fn exfat_refactor_same_image_block_probe(nixos_shell: &mut Session) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect("cat /proc/filesystems", "exfat_refactor")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;
    write_guest_file(
        nixos_shell,
        "pb",
        &[
            "import fcntl",
            "import os",
            "import struct",
            "import traceback",
            "",
            "BLKSSZGET = 0x1268",
            "BLKGETSIZE64 = 0x80081272",
            "",
            "def print_kv(key, value):",
            "    print(f\"{key}={value}\")",
            "",
            "def ioctl_u32(fd, request):",
            "    buffer = bytearray(4)",
            "    fcntl.ioctl(fd, request, buffer, True)",
            "    return struct.unpack(\"<I\", buffer)[0]",
            "",
            "def ioctl_u64(fd, request):",
            "    buffer = bytearray(8)",
            "    fcntl.ioctl(fd, request, buffer, True)",
            "    return struct.unpack(\"<Q\", buffer)[0]",
            "",
            "def safe_call(label, call_fn):",
            "    try:",
            "        print_kv(label, call_fn())",
            "    except Exception as exc:",
            "        print_kv(label, f\"ERR:{exc!r}\")",
            "",
            "def le32(data, offset):",
            "    return struct.unpack_from(\"<I\", data, offset)[0]",
            "",
            "def le64(data, offset):",
            "    return struct.unpack_from(\"<Q\", data, offset)[0]",
            "",
            "path = os.environ.get(\"XFSTESTS_TEST_DEV\", \"/dev/vdb\")",
            "fd = None",
            "rc = 0",
            "print(\"BLOCK_PROBE_BEGIN\")",
            "try:",
            "    fd = os.open(path, os.O_RDONLY)",
            "    boot = os.pread(fd, 512, 0)",
            "    bytes_per_sector_shift = boot[0x6C]",
            "    sectors_per_cluster_shift = boot[0x6D]",
            "    bytes_per_sector = 1 << bytes_per_sector_shift",
            "    sectors_per_cluster = 1 << sectors_per_cluster_shift",
            "    fat_offset_sectors = le32(boot, 0x50)",
            "    fat_length_sectors = le32(boot, 0x54)",
            "    cluster_heap_offset_sectors = le32(boot, 0x58)",
            "    cluster_count = le32(boot, 0x5C)",
            "    root_cluster = le32(boot, 0x60)",
            "    fat_offset_bytes = fat_offset_sectors * bytes_per_sector",
            "    root_fat_entry_offset = fat_offset_bytes + root_cluster * 4",
            "    root_dir_offset = (cluster_heap_offset_sectors + ((root_cluster - 2) * sectors_per_cluster)) * bytes_per_sector",
            "    print_kv(\"BLOCK_PROBE_PATH\", path)",
            "    safe_call(\"BLOCK_PROBE_FSTAT_SIZE\", lambda: os.fstat(fd).st_size)",
            "    safe_call(\"BLOCK_PROBE_LSEEK_END\", lambda: os.lseek(fd, 0, os.SEEK_END))",
            "    os.lseek(fd, 0, os.SEEK_SET)",
            "    safe_call(\"BLOCK_PROBE_IOCTL_BLKSSZGET\", lambda: ioctl_u32(fd, BLKSSZGET))",
            "    safe_call(\"BLOCK_PROBE_IOCTL_BLKGETSIZE64\", lambda: ioctl_u64(fd, BLKGETSIZE64))",
            "    print_kv(\"BLOCK_PROBE_BOOT_HEX_0_64\", os.pread(fd, 64, 0).hex())",
            "    print_kv(\"BLOCK_PROBE_BOOT_HEX_0_64_REPEAT\", os.pread(fd, 64, 0).hex())",
            "    print_kv(\"BLOCK_PROBE_BPB_VOLUME_LENGTH\", le64(boot, 0x48))",
            "    print_kv(\"BLOCK_PROBE_BPB_FAT_OFFSET_SECTORS\", fat_offset_sectors)",
            "    print_kv(\"BLOCK_PROBE_BPB_FAT_LENGTH_SECTORS\", fat_length_sectors)",
            "    print_kv(\"BLOCK_PROBE_BPB_CLUSTER_HEAP_OFFSET_SECTORS\", cluster_heap_offset_sectors)",
            "    print_kv(\"BLOCK_PROBE_BPB_CLUSTER_COUNT\", cluster_count)",
            "    print_kv(\"BLOCK_PROBE_BPB_ROOT_CLUSTER\", root_cluster)",
            "    print_kv(\"BLOCK_PROBE_BPB_BYTES_PER_SECTOR_SHIFT\", bytes_per_sector_shift)",
            "    print_kv(\"BLOCK_PROBE_BPB_SECTORS_PER_CLUSTER_SHIFT\", sectors_per_cluster_shift)",
            "    print_kv(\"BLOCK_PROBE_BPB_BYTES_PER_SECTOR\", bytes_per_sector)",
            "    print_kv(\"BLOCK_PROBE_BPB_SECTORS_PER_CLUSTER\", sectors_per_cluster)",
            "    print_kv(\"BLOCK_PROBE_FAT_OFFSET_BYTES\", fat_offset_bytes)",
            "    print_kv(\"BLOCK_PROBE_ROOT_FAT_ENTRY_OFFSET\", root_fat_entry_offset)",
            "    print_kv(\"BLOCK_PROBE_ROOT_DIR_OFFSET\", root_dir_offset)",
            "    print_kv(\"BLOCK_PROBE_FAT_HEX_ROOT_ENTRY\", os.pread(fd, 16, root_fat_entry_offset).hex())",
            "    print_kv(\"BLOCK_PROBE_FAT_HEX_ROOT_ENTRY_REPEAT\", os.pread(fd, 16, root_fat_entry_offset).hex())",
            "    print_kv(\"BLOCK_PROBE_ROOT_DIR_HEX_0_64\", os.pread(fd, 64, root_dir_offset).hex())",
            "    print_kv(\"BLOCK_PROBE_ROOT_DIR_HEX_0_64_REPEAT\", os.pread(fd, 64, root_dir_offset).hex())",
            "except Exception as exc:",
            "    rc = 1",
            "    print_kv(\"BLOCK_PROBE_EXCEPTION\", repr(exc))",
            "    traceback.print_exc()",
            "finally:",
            "    if fd is not None:",
            "        os.close(fd)",
            "print_kv(\"BLOCK_PROBE_INTERNAL_RC\", rc)",
            "print(\"BLOCK_PROBE_DONE\")",
        ],
    )?;
    nixos_shell.run_cmd("chmod +x /tmp/pb")?;
    nixos_shell.run_cmd_and_expect(
        &format!(
            "XFSTESTS_TEST_DEV={} /nix/var/nix/profiles/system/sw/bin/python3 /tmp/pb",
            test_dev
        ),
        "BLOCK_PROBE_DONE",
    )?;

    Ok(())
}

fn run_python_block_probe(
    nixos_shell: &mut Session,
    script_name: &str,
    script_lines: &[&str],
    done_marker: &str,
) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect("cat /proc/filesystems", "exfat_refactor")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;
    write_guest_file(nixos_shell, script_name, script_lines)?;
    nixos_shell.run_cmd(&format!("chmod +x /tmp/{script_name}"))?;
    nixos_shell.run_cmd_and_expect(
        &format!(
            "XFSTESTS_TEST_DEV={} /nix/var/nix/profiles/system/sw/bin/python3 /tmp/{}",
            test_dev, script_name
        ),
        done_marker,
    )?;

    Ok(())
}

#[nixos_test]
fn exfat_refactor_same_image_blkpbszget_probe(nixos_shell: &mut Session) -> Result<(), Error> {
    run_python_block_probe(
        nixos_shell,
        "ppbs",
        &[
            "import fcntl",
            "import os",
            "import struct",
            "import traceback",
            "",
            "BLKPBSZGET = 0x127b",
            "",
            "def print_kv(key, value):",
            "    print(f\"{key}={value}\")",
            "",
            "def ioctl_u32(fd, request):",
            "    buffer = bytearray(4)",
            "    fcntl.ioctl(fd, request, buffer, True)",
            "    return struct.unpack(\"<I\", buffer)[0]",
            "",
            "path = os.environ.get(\"XFSTESTS_TEST_DEV\", \"/dev/vdb\")",
            "fd = None",
            "rc = 0",
            "print(\"BLKPBSZGET_PROBE_BEGIN\")",
            "try:",
            "    fd = os.open(path, os.O_RDONLY)",
            "    print_kv(\"BLKPBSZGET_PROBE_RESULT\", ioctl_u32(fd, BLKPBSZGET))",
            "except Exception as exc:",
            "    rc = 1",
            "    print_kv(\"BLKPBSZGET_PROBE_EXCEPTION\", repr(exc))",
            "    traceback.print_exc()",
            "finally:",
            "    if fd is not None:",
            "        os.close(fd)",
            "print_kv(\"BLKPBSZGET_PROBE_RC\", rc)",
            "print(\"BLKPBSZGET_PROBE_DONE\")",
        ],
        "BLKPBSZGET_PROBE_DONE",
    )
}

#[nixos_test]
fn exfat_refactor_same_image_pread_512_at_zero_probe(
    nixos_shell: &mut Session,
) -> Result<(), Error> {
    run_python_block_probe(
        nixos_shell,
        "pp512",
        &[
            "import os",
            "import traceback",
            "",
            "path = os.environ.get(\"XFSTESTS_TEST_DEV\", \"/dev/vdb\")",
            "fd = None",
            "rc = 0",
            "print(\"PREAD_512_AT_ZERO_PROBE_BEGIN\")",
            "try:",
            "    fd = os.open(path, os.O_RDONLY)",
            "    print(f\"PREAD_512_AT_ZERO_PROBE_HEX={os.pread(fd, 512, 0).hex()}\")",
            "except Exception as exc:",
            "    rc = 1",
            "    print(f\"PREAD_512_AT_ZERO_PROBE_EXCEPTION={exc!r}\")",
            "    traceback.print_exc()",
            "finally:",
            "    if fd is not None:",
            "        os.close(fd)",
            "print(f\"PREAD_512_AT_ZERO_PROBE_RC={rc}\")",
            "print(\"PREAD_512_AT_ZERO_PROBE_DONE\")",
        ],
        "PREAD_512_AT_ZERO_PROBE_DONE",
    )
}

#[nixos_test]
fn exfat_refactor_same_image_pread_64_at_zero_probe(
    nixos_shell: &mut Session,
) -> Result<(), Error> {
    run_python_block_probe(
        nixos_shell,
        "pp640",
        &[
            "import os",
            "import traceback",
            "",
            "path = os.environ.get(\"XFSTESTS_TEST_DEV\", \"/dev/vdb\")",
            "fd = None",
            "rc = 0",
            "print(\"PREAD_64_AT_ZERO_PROBE_BEGIN\")",
            "try:",
            "    fd = os.open(path, os.O_RDONLY)",
            "    print(f\"PREAD_64_AT_ZERO_PROBE_HEX={os.pread(fd, 64, 0).hex()}\")",
            "except Exception as exc:",
            "    rc = 1",
            "    print(f\"PREAD_64_AT_ZERO_PROBE_EXCEPTION={exc!r}\")",
            "    traceback.print_exc()",
            "finally:",
            "    if fd is not None:",
            "        os.close(fd)",
            "print(f\"PREAD_64_AT_ZERO_PROBE_RC={rc}\")",
            "print(\"PREAD_64_AT_ZERO_PROBE_DONE\")",
        ],
        "PREAD_64_AT_ZERO_PROBE_DONE",
    )
}

#[nixos_test]
fn exfat_refactor_same_image_pread_root_fat_entry_probe(
    nixos_shell: &mut Session,
) -> Result<(), Error> {
    run_python_block_probe(
        nixos_shell,
        "ppfat",
        &[
            "import os",
            "import traceback",
            "",
            "ROOT_FAT_ENTRY_OFFSET = 1048592",
            "",
            "path = os.environ.get(\"XFSTESTS_TEST_DEV\", \"/dev/vdb\")",
            "fd = None",
            "rc = 0",
            "print(\"PREAD_ROOT_FAT_ENTRY_PROBE_BEGIN\")",
            "try:",
            "    fd = os.open(path, os.O_RDONLY)",
            "    print(f\"PREAD_ROOT_FAT_ENTRY_PROBE_HEX={os.pread(fd, 4, ROOT_FAT_ENTRY_OFFSET).hex()}\")",
            "except Exception as exc:",
            "    rc = 1",
            "    print(f\"PREAD_ROOT_FAT_ENTRY_PROBE_EXCEPTION={exc!r}\")",
            "    traceback.print_exc()",
            "finally:",
            "    if fd is not None:",
            "        os.close(fd)",
            "print(f\"PREAD_ROOT_FAT_ENTRY_PROBE_RC={rc}\")",
            "print(\"PREAD_ROOT_FAT_ENTRY_PROBE_DONE\")",
        ],
        "PREAD_ROOT_FAT_ENTRY_PROBE_DONE",
    )
}

#[nixos_test]
fn exfat_refactor_same_image_aligned_sector_probe(
    nixos_shell: &mut Session,
) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect("cat /proc/filesystems", "exfat_refactor")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;
    write_guest_file(
        nixos_shell,
        "pasp",
        &[
            "import fcntl",
            "import os",
            "import struct",
            "import traceback",
            "",
            "BLKSSZGET = 0x1268",
            "BLKGETSIZE64 = 0x80081272",
            "SECTOR_SIZE = 512",
            "",
            "def print_kv(key, value):",
            "    print(f\"{key}={value}\")",
            "",
            "def ioctl_u32(fd, request):",
            "    buffer = bytearray(4)",
            "    fcntl.ioctl(fd, request, buffer, True)",
            "    return struct.unpack(\"<I\", buffer)[0]",
            "",
            "def ioctl_u64(fd, request):",
            "    buffer = bytearray(8)",
            "    fcntl.ioctl(fd, request, buffer, True)",
            "    return struct.unpack(\"<Q\", buffer)[0]",
            "",
            "def le32(data, offset):",
            "    return struct.unpack_from(\"<I\", data, offset)[0]",
            "",
            "path = os.environ.get(\"XFSTESTS_TEST_DEV\", \"/dev/vdb\")",
            "fd = None",
            "rc = 0",
            "print(\"ALIGNED_SECTOR_PROBE_BEGIN\")",
            "try:",
            "    fd = os.open(path, os.O_RDONLY)",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_LSEEK_END\", os.lseek(fd, 0, os.SEEK_END))",
            "    os.lseek(fd, 0, os.SEEK_SET)",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_IOCTL_BLKSSZGET\", ioctl_u32(fd, BLKSSZGET))",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_IOCTL_BLKGETSIZE64\", ioctl_u64(fd, BLKGETSIZE64))",
            "    sector0 = os.pread(fd, SECTOR_SIZE, 0)",
            "    bytes_per_sector_shift = sector0[0x6C]",
            "    sectors_per_cluster_shift = sector0[0x6D]",
            "    bytes_per_sector = 1 << bytes_per_sector_shift",
            "    sectors_per_cluster = 1 << sectors_per_cluster_shift",
            "    fat_offset_sectors = le32(sector0, 0x50)",
            "    cluster_heap_offset_sectors = le32(sector0, 0x58)",
            "    root_cluster = le32(sector0, 0x60)",
            "    fat_offset_bytes = fat_offset_sectors * bytes_per_sector",
            "    root_fat_entry_offset = fat_offset_bytes + root_cluster * 4",
            "    root_fat_sector_offset = (root_fat_entry_offset // SECTOR_SIZE) * SECTOR_SIZE",
            "    root_dir_offset = (cluster_heap_offset_sectors + ((root_cluster - 2) * sectors_per_cluster)) * bytes_per_sector",
            "    root_dir_sector_offset = (root_dir_offset // SECTOR_SIZE) * SECTOR_SIZE",
            "    root_fat_sector = os.pread(fd, SECTOR_SIZE, root_fat_sector_offset)",
            "    root_dir_sector = os.pread(fd, SECTOR_SIZE, root_dir_sector_offset)",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_BPB_ROOT_CLUSTER\", root_cluster)",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_ROOT_FAT_ENTRY_OFFSET\", root_fat_entry_offset)",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_ROOT_FAT_SECTOR_OFFSET\", root_fat_sector_offset)",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_ROOT_DIR_OFFSET\", root_dir_offset)",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_ROOT_DIR_SECTOR_OFFSET\", root_dir_sector_offset)",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_SECTOR0_HEX\", sector0.hex())",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_ROOT_FAT_SECTOR_HEX\", root_fat_sector.hex())",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_ROOT_DIR_SECTOR_HEX\", root_dir_sector.hex())",
            "except Exception as exc:",
            "    rc = 1",
            "    print_kv(\"ALIGNED_SECTOR_PROBE_EXCEPTION\", repr(exc))",
            "    traceback.print_exc()",
            "finally:",
            "    if fd is not None:",
            "        os.close(fd)",
            "print_kv(\"ALIGNED_SECTOR_PROBE_INTERNAL_RC\", rc)",
            "print(\"ALIGNED_SECTOR_PROBE_DONE\")",
        ],
    )?;
    nixos_shell.run_cmd("chmod +x /tmp/pasp")?;
    nixos_shell.run_cmd_and_expect(
        &format!(
            "XFSTESTS_TEST_DEV={} /nix/var/nix/profiles/system/sw/bin/python3 /tmp/pasp",
            test_dev
        ),
        "ALIGNED_SECTOR_PROBE_DONE",
    )?;

    Ok(())
}

#[nixos_test]
fn exfat_refactor_fsck_same_image_strace_probe(nixos_shell: &mut Session) -> Result<(), Error> {
    let test_dev = env::var("XFSTESTS_TEST_DEV").unwrap_or_else(|_| "/dev/vdb".to_string());

    nixos_shell.run_cmd_and_expect("cat /proc/filesystems", "exfat_refactor")?;
    nixos_shell.run_cmd_and_expect(&format!("test -b {}&&echo td", test_dev), "td")?;
    nixos_shell.run_cmd("cd /tmp")?;
    write_guest_file(
        nixos_shell,
        "pfs",
        &[
            "#!/bin/sh",
            "set +e",
            "test_dev=${XFSTESTS_TEST_DEV:-/dev/vdb}",
            "export PATH=/nix/var/nix/profiles/system/sw/bin:$PATH",
            "helper=/nix/var/nix/profiles/system/sw/bin/fsck.exfat",
            "strace_helper=/nix/var/nix/profiles/system/sw/bin/strace",
            "trace_file=/tmp/exfat-probe-fsck.strace",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/exfat-strace-probe-mounted-before.txt 2>&1 || true",
            "if [ -x \"$helper\" ] && [ -x \"$strace_helper\" ]; then",
            "\"$strace_helper\" -o \"$trace_file\" -yy -s 128 -e trace=pread64,read,readv,preadv,preadv2,lseek,ioctl \"$helper\" -n \"$test_dev\" >/tmp/exfat-strace-probe-fsck.out 2>&1",
            "rc=$?",
            "elif [ ! -x \"$helper\" ]; then",
            "printf 'missing exFAT checker: %s\\n' \"$helper\" >/tmp/exfat-strace-probe-fsck.out",
            "rc=127",
            "else",
            "printf 'missing strace helper: %s\\n' \"$strace_helper\" >/tmp/exfat-strace-probe-fsck.out",
            "rc=126",
            "fi",
            "findmnt -rncv -S \"$test_dev\" -o SOURCE,FSTYPE,TARGET >/tmp/exfat-strace-probe-mounted-after.txt 2>&1 || true",
            "echo EXFAT_STRACE_PROBE_RC=$rc",
            "echo EXFAT_STRACE_PROBE_OUTPUT_BEGIN",
            "cat /tmp/exfat-strace-probe-fsck.out",
            "echo EXFAT_STRACE_PROBE_OUTPUT_END",
            "echo EXFAT_STRACE_PROBE_TRACE_BEGIN",
            "cat \"$trace_file\" 2>/dev/null || true",
            "echo EXFAT_STRACE_PROBE_TRACE_END",
            "echo EXFAT_STRACE_PROBE_MOUNTED_BEFORE_BEGIN",
            "cat /tmp/exfat-strace-probe-mounted-before.txt",
            "echo EXFAT_STRACE_PROBE_MOUNTED_BEFORE_END",
            "echo EXFAT_STRACE_PROBE_MOUNTED_AFTER_BEGIN",
            "cat /tmp/exfat-strace-probe-mounted-after.txt",
            "echo EXFAT_STRACE_PROBE_MOUNTED_AFTER_END",
            "echo EXFAT_STRACE_PROBE_DONE",
            "exit 0",
        ],
    )?;
    nixos_shell.run_cmd("chmod +x /tmp/pfs")?;
    nixos_shell.run_cmd_and_expect(
        &format!("XFSTESTS_TEST_DEV={} /tmp/pfs", test_dev),
        "EXFAT_STRACE_PROBE_DONE",
    )?;

    Ok(())
}
