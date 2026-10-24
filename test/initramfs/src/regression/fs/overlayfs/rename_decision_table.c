// SPDX-License-Identifier: MPL-2.0

#define _GNU_SOURCE
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <sys/xattr.h>
#include <unistd.h>

#include "../../common/test.h"

#define BASE_DIR "/ovl_rename_decision_table"
#define UPPER_DIR BASE_DIR "/upper"
#define WORK_DIR BASE_DIR "/work"
#define LOWER_DIR BASE_DIR "/lower"
#define MERGED_DIR BASE_DIR "/merged"

#define RL LOWER_DIR
#define RM MERGED_DIR
#define RU UPPER_DIR

#define PU_DIR "/pu"
#define PM_DIR "/pm"
#define PE_DIR "/pe"
#define PV_DIR "/pv"
#define PL_DIR "/pl"

#define OPAQUE_XATTR "trusted.overlay.opaque"
#define IMPURE_XATTR "trusted.overlay.impure"
#define ORIGIN_XATTR "trusted.overlay.origin"

#define MODE_REPLACE 0
#define MODE_NOREPLACE RENAME_NOREPLACE
#define MODE_EXCHANGE RENAME_EXCHANGE

#define S_C01 "s01"
#define T_C01 "t01"
#define S_C02 "s02"
#define T_C02 "t02"
#define S_C03 "s03"
#define T_C03 "t03"
#define X_C03 "x_c03"
#define S_C04 "s04"
#define T_C04 "t04"
#define X_C04 "x_c04"
#define S_C05 "s05"
#define T_C05 "t05"
#define S_C06 "s06"
#define T_C06 "t06"
#define S_C07 "s07"
#define T_C07 "t07"
#define S_C08 "s08"
#define T_C08 "t08"
#define X_C08 "x_c08"
#define S_C09 "s09"
#define T_C09 "t09"
#define X_C09 "x_c09"
#define S_C10 "s10"
#define T_C10 "t10"
#define X_C10 "x_c10"
#define S_C11 "s11"
#define T_C11 "t11"
#define S_C12 "s12"
#define T_C12 "t12"
#define S_C13 "s13"
#define T_C13 "t13"
#define S_C14 "s14"
#define T_C14 "t14"
#define X_C14 "x_c14"
#define S_C15 "s15"
#define T_C15 "t15"
#define X_C15 "x_c15"
#define S_C16 "s16"
#define T_C16 "t16"
#define X_C16 "x_c16"
#define S_C17 "s17"
#define T_C17 "t17"
#define S_C18 "s18"
#define T_C18 "t18"
#define S_C19 "s19"
#define T_C19 "t19"
#define S_C20 "s20"
#define T_C20 "t20"
#define X_C20 "x_c20"
#define S_C21 "s21"
#define T_C21 "t21"
#define X_C21 "x_c21"
#define S_C22 "s22"
#define T_C22 "t22"
#define X_C22 "x_c22"
#define S_C23 "s23"
#define T_C23 "t23"
#define S_C24 "s24"
#define T_C24 "t24"
#define S_C25 "s25"
#define T_C25 "t25"
#define X_C25 "x_c25"
#define S_C26 "s26"
#define T_C26 "t26"
#define X_C26 "x_c26"
#define S_C27 "s27"
#define T_C27 "t27"
#define X_C27 "x_c27"
#define S_C28 "s28"
#define T_C28 "t28"
#define X_C28 "x_c28"
#define X_C28B "x_c28b"
#define S_C29 "s29"
#define T_C29 "t29"
#define S_C30 "s30"
#define T_C30 "t30"
#define S_C31 "s31"
#define T_C31 "t31"
#define X_C31 "x_c31"
#define S_C32 "s32"
#define T_C32 "t32"
#define X_C32 "x_c32"

#define S_X01 "xs01"
#define T_X01 "xt01"
#define X_X01 "x_x01"
#define S_X02 "xs02"
#define T_X02 "xt02"
#define S_X03 "xs03"
#define T_X03 "xt03"
#define S_X04 "xs04"
#define T_X04 "xt04"
#define X_X04A "x_x04a"
#define X_X04B "x_x04b"
#define S_X05 "xs05"
#define T_X05 "xt05"
#define X_X05A "x_x05a"
#define X_X05B "x_x05b"
#define S_X06 "xs06"
#define T_X06 "xt06"
#define X_X06 "x_x06"
#define S_X07 "xs07"
#define T_X07 "xt07"
#define S_X08 "xs08"
#define T_X08 "xt08"
#define S_X09 "xs09"
#define T_X09 "xt09"
#define X_X09A "x_x09a"
#define X_X09B "x_x09b"
#define S_X10 "xs10"
#define T_X10 "xt10"
#define S_X11 "xs11"
#define T_X11 "xt11"
#define X_X11 "x_x11"
#define S_X12 "xs12"
#define T_X12 "xt12"
#define X_X12 "x_x12"
#define X_X12B "x_x12b"
#define S_X13 "xs13"
#define T_X13 "xt13"
#define T_X13B "xt13b"

#define DEC_A "dec_a"
#define DEC_B "dec_b"

#define SYMLINK_TARGET "nowhere_target"

static void make_dir(const char *path)
{
	CHECK(mkdir(path, 0755));
}

static void make_file(const char *path, const char *content)
{
	int fd = CHECK(open(path, O_CREAT | O_WRONLY, 0644));

	CHECK(write(fd, content, strlen(content)));
	CHECK(close(fd));
}

static void make_symlink(const char *path, const char *target)
{
	CHECK(symlink(target, path));
}

static void make_whiteout(const char *path)
{
	CHECK(mknod(path, S_IFCHR | 0644, makedev(0, 0)));
}

static void set_marker(const char *path, const char *name)
{
	CHECK(setxattr(path, name, "y", 1, 0));
}

static void merge_path(char *buf, size_t size, const char *dir,
		       const char *name)
{
	int ret = snprintf(buf, size, "%s/%s", dir, name);

	CHECK_WITH(ret, _ret >= 0 && (size_t)_ret < size);
}

static void physical_path(char *buf, size_t size, const char *layer,
			  const char *dir, const char *name)
{
	int ret = snprintf(buf, size, "%s%s/%s", layer, dir, name);

	CHECK_WITH(ret, _ret >= 0 && (size_t)_ret < size);
}

static int find_entry(const char *dir_path, const char *name)
{
	DIR *dir = opendir(dir_path);
	struct dirent *entry;
	int found = 0;

	if (dir == NULL)
		return -1;
	for (;;) {
		errno = 0;
		entry = readdir(dir);
		if (entry == NULL)
			break;
		if (strcmp(entry->d_name, ".") == 0 ||
		    strcmp(entry->d_name, "..") == 0)
			continue;
		if (strcmp(entry->d_name, name) == 0) {
			found = 1;
			break;
		}
	}
	if (errno != 0)
		found = -1;
	closedir(dir);
	return found;
}

static int count_entries(const char *dir_path)
{
	DIR *dir = opendir(dir_path);
	struct dirent *entry;
	int count = 0;

	if (dir == NULL)
		return -1;
	for (;;) {
		errno = 0;
		entry = readdir(dir);
		if (entry == NULL)
			break;
		if (strcmp(entry->d_name, ".") == 0 ||
		    strcmp(entry->d_name, "..") == 0)
			continue;
		count++;
	}
	if (errno != 0)
		count = -1;
	closedir(dir);
	return count;
}

static int count_char_dev_zero(const char *dir_path)
{
	DIR *dir = opendir(dir_path);
	struct dirent *entry;
	struct stat st;
	char path[PATH_MAX];
	int count = 0;

	if (dir == NULL)
		return -1;
	for (;;) {
		errno = 0;
		entry = readdir(dir);
		if (entry == NULL)
			break;
		if (strcmp(entry->d_name, ".") == 0 ||
		    strcmp(entry->d_name, "..") == 0)
			continue;
		merge_path(path, sizeof(path), dir_path, entry->d_name);
		if (lstat(path, &st) != 0)
			continue;
		if (S_ISCHR(st.st_mode) && st.st_rdev == 0)
			count++;
	}
	if (errno != 0)
		count = -1;
	closedir(dir);
	return count;
}

static int count_opaque_dirs(const char *dir_path)
{
	DIR *dir = opendir(dir_path);
	struct dirent *entry;
	struct stat st;
	char path[PATH_MAX];
	char value[64];
	int count = 0;

	if (dir == NULL)
		return -1;
	for (;;) {
		errno = 0;
		entry = readdir(dir);
		if (entry == NULL)
			break;
		if (strcmp(entry->d_name, ".") == 0 ||
		    strcmp(entry->d_name, "..") == 0)
			continue;
		merge_path(path, sizeof(path), dir_path, entry->d_name);
		if (lstat(path, &st) != 0 || !S_ISDIR(st.st_mode))
			continue;
		if (getxattr(path, OPAQUE_XATTR, value, sizeof(value)) == 1 &&
		    value[0] == 'y')
			count++;
	}
	if (errno != 0)
		count = -1;
	closedir(dir);
	return count;
}

static void read_file_body(const char *path, char *buf, size_t size)
{
	int fd = open(path, O_RDONLY);
	ssize_t ret;
	size_t used = 0;

	buf[0] = '\0';
	if (fd < 0)
		return;
	while (used + 1 < size) {
		ret = read(fd, buf + used, size - used - 1);
		if (ret <= 0)
			break;
		used += (size_t)ret;
	}
	buf[used] = '\0';
	close(fd);
}

static void remove_tree(const char *path)
{
	struct stat st;

	if (lstat(path, &st) != 0)
		return;
	if (!S_ISDIR(st.st_mode)) {
		unlink(path);
		return;
	}

	DIR *dir = opendir(path);
	if (dir == NULL) {
		rmdir(path);
		return;
	}

	struct dirent *entry;
	while ((entry = readdir(dir)) != NULL) {
		char child[PATH_MAX];
		int ret;

		if (strcmp(entry->d_name, ".") == 0 ||
		    strcmp(entry->d_name, "..") == 0)
			continue;
		ret = snprintf(child, sizeof(child), "%s/%s", path,
			       entry->d_name);
		if (ret < 0 || (size_t)ret >= sizeof(child))
			continue;
		remove_tree(child);
	}

	closedir(dir);
	rmdir(path);
}

static void build_lower_tree(void)
{
	make_dir(LOWER_DIR "/pm");
	make_dir(LOWER_DIR "/pe");
	make_dir(LOWER_DIR "/pl");

	make_dir(LOWER_DIR "/pm/t12");
	make_dir(LOWER_DIR "/pm/t13");
	make_dir(LOWER_DIR "/pm/t14");
	make_dir(LOWER_DIR "/pm/t15");
	make_dir(LOWER_DIR "/pm/t16");
	make_dir(LOWER_DIR "/pm/" DEC_A);
	make_dir(LOWER_DIR "/pm/xt09");
	make_file(LOWER_DIR "/pm/xt09/" X_X09B, X_X09B);
	make_dir(LOWER_DIR "/pm/xs04");
	make_file(LOWER_DIR "/pm/xs04/" X_X04B, X_X04B);
	make_dir(LOWER_DIR "/pm/xs05");
	make_file(LOWER_DIR "/pm/xs05/" X_X05B, X_X05B);
	make_file(LOWER_DIR "/pm/t05", "low05");
	make_file(LOWER_DIR "/pm/t06", "low06");
	make_file(LOWER_DIR "/pm/t09", "low09");
	make_file(LOWER_DIR "/pm/" DEC_B, DEC_B);
	make_file(LOWER_DIR "/pm/xt07", "low07y");
	make_file(LOWER_DIR "/pm/xs02", "low02");
	make_file(LOWER_DIR "/pm/xs07", "low07x");
	make_file(LOWER_DIR "/pm/xs08", "low08");
	make_file(LOWER_DIR "/pm/xs10", "low10");
	make_file(LOWER_DIR "/pm/xs11", "low11");
	make_file(LOWER_DIR "/pm/xs13", "low13");
	make_whiteout(LOWER_DIR "/pm/xt01");

	make_file(LOWER_DIR "/pe/t07", "low07");
	make_file(LOWER_DIR "/pe/t10", "low10");

	make_file(LOWER_DIR "/pl/t08", "low08");
	make_dir(LOWER_DIR "/pl/t11");
	make_dir(LOWER_DIR "/pl/xt12");
	make_file(LOWER_DIR "/pl/xt12/" X_X12B, X_X12B);
	make_dir(LOWER_DIR "/pl/xs06");
	make_file(LOWER_DIR "/pl/xs06/" X_X06, X_X06);
}

static void build_upper_tree(void)
{
	make_dir(UPPER_DIR "/pu");
	make_dir(UPPER_DIR "/pm");
	make_dir(UPPER_DIR "/pe");
	make_dir(UPPER_DIR "/pv");

	make_file(UPPER_DIR "/pu/s01", "src01");
	make_file(UPPER_DIR "/pu/s02", "src02");
	make_dir(UPPER_DIR "/pu/s03");
	make_file(UPPER_DIR "/pu/s03/" X_C03, X_C03);
	make_dir(UPPER_DIR "/pu/s04");
	make_file(UPPER_DIR "/pu/s04/" X_C04, X_C04);
	make_file(UPPER_DIR "/pu/s07", "src07");
	make_dir(UPPER_DIR "/pu/s08");
	make_file(UPPER_DIR "/pu/s08/" X_C08, X_C08);
	make_dir(UPPER_DIR "/pu/s10");
	make_file(UPPER_DIR "/pu/s10/" X_C10, X_C10);
	make_file(UPPER_DIR "/pu/s11", "src11");
	make_file(UPPER_DIR "/pu/s13", "src13");
	make_dir(UPPER_DIR "/pu/s16");
	make_file(UPPER_DIR "/pu/s16/" X_C16, X_C16);
	make_file(UPPER_DIR "/pu/s17", "src17");
	make_file(UPPER_DIR "/pu/t17", "old17");
	make_file(UPPER_DIR "/pu/s18", "src18");
	make_file(UPPER_DIR "/pu/t18", "old18");
	make_file(UPPER_DIR "/pu/s19", "src19");
	make_file(UPPER_DIR "/pu/t19", "old19");
	make_dir(UPPER_DIR "/pu/s20");
	make_file(UPPER_DIR "/pu/s20/" X_C20, X_C20);
	make_file(UPPER_DIR "/pu/t20", "old20");
	make_dir(UPPER_DIR "/pu/s21");
	make_file(UPPER_DIR "/pu/s21/" X_C21, X_C21);
	make_file(UPPER_DIR "/pu/t21", "old21");
	make_dir(UPPER_DIR "/pu/s22");
	make_file(UPPER_DIR "/pu/s22/" X_C22, X_C22);
	make_file(UPPER_DIR "/pu/t22", "old22");
	make_file(UPPER_DIR "/pu/s23", "src23");
	make_dir(UPPER_DIR "/pu/t23");
	make_file(UPPER_DIR "/pu/s24", "src24");
	make_dir(UPPER_DIR "/pu/t24");
	make_file(UPPER_DIR "/pu/s25", "src25");
	make_dir(UPPER_DIR "/pu/t25");
	make_file(UPPER_DIR "/pu/t25/" X_C25, X_C25);
	make_dir(UPPER_DIR "/pu/s27");
	make_file(UPPER_DIR "/pu/s27/" X_C27, X_C27);
	make_dir(UPPER_DIR "/pu/t27");
	make_dir(UPPER_DIR "/pu/s28");
	make_file(UPPER_DIR "/pu/s28/" X_C28, X_C28);
	make_dir(UPPER_DIR "/pu/t28");
	make_file(UPPER_DIR "/pu/t28/" X_C28B, X_C28B);
	make_file(UPPER_DIR "/pu/s29", "src29");
	make_file(UPPER_DIR "/pu/s30", "src30");
	make_dir(UPPER_DIR "/pu/s31");
	make_file(UPPER_DIR "/pu/s31/" X_C31, X_C31);
	make_dir(UPPER_DIR "/pu/s32");
	make_file(UPPER_DIR "/pu/s32/" X_C32, X_C32);
	make_whiteout(UPPER_DIR "/pu/t29");
	make_whiteout(UPPER_DIR "/pu/t30");
	make_whiteout(UPPER_DIR "/pu/t31");
	make_whiteout(UPPER_DIR "/pu/t32");
	make_dir(UPPER_DIR "/pu/xs01");
	make_file(UPPER_DIR "/pu/xs01/" X_X01, X_X01);
	make_dir(UPPER_DIR "/pu/xs12");
	make_file(UPPER_DIR "/pu/xs12/" X_X12, X_X12);

	make_file(UPPER_DIR "/pm/s05", "src05");
	make_file(UPPER_DIR "/pm/s06", "src06");
	make_dir(UPPER_DIR "/pm/s09");
	make_file(UPPER_DIR "/pm/s09/" X_C09, X_C09);
	make_file(UPPER_DIR "/pm/s12", "src12");
	make_dir(UPPER_DIR "/pm/s14");
	make_file(UPPER_DIR "/pm/s14/" X_C14, X_C14);
	make_dir(UPPER_DIR "/pm/t14");
	make_dir(UPPER_DIR "/pm/s15");
	make_file(UPPER_DIR "/pm/s15/" X_C15, X_C15);
	make_dir(UPPER_DIR "/pm/t15");
	make_dir(UPPER_DIR "/pm/t16");
	make_dir(UPPER_DIR "/pm/s26");
	make_file(UPPER_DIR "/pm/s26/" X_C26, X_C26);
	make_dir(UPPER_DIR "/pm/t26");
	make_file(UPPER_DIR "/pm/xs02", "up02");
	make_dir(UPPER_DIR "/pm/xs04");
	make_file(UPPER_DIR "/pm/xs04/" X_X04A, X_X04A);
	make_dir(UPPER_DIR "/pm/xs05");
	make_file(UPPER_DIR "/pm/xs05/" X_X05A, X_X05A);
	make_file(UPPER_DIR "/pm/xs07", "src07x");
	make_file(UPPER_DIR "/pm/xs08", "src08");
	make_dir(UPPER_DIR "/pm/xs09");
	make_file(UPPER_DIR "/pm/xs09/" X_X09A, X_X09A);
	make_dir(UPPER_DIR "/pm/xt09");
	make_file(UPPER_DIR "/pm/xs10", "src10");
	make_whiteout(UPPER_DIR "/pm/xt10");
	make_dir(UPPER_DIR "/pm/xs11");
	make_file(UPPER_DIR "/pm/xs11/" X_X11, X_X11);
	make_whiteout(UPPER_DIR "/pm/xt11");

	set_marker(UPPER_DIR "/pm/xs04", OPAQUE_XATTR);
	set_marker(UPPER_DIR "/pm/xt09", OPAQUE_XATTR);
	set_marker(UPPER_DIR "/pv", IMPURE_XATTR);
	make_symlink(UPPER_DIR "/pu/xs03", SYMLINK_TARGET);
}

static void verify_whiteout(const char *path)
{
	struct stat st;

	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	CHECK_WITH(st.st_rdev, _ret == 0);
}

static void verify_marker(const char *path, const char *name)
{
	char value[64];

	CHECK_WITH(getxattr(path, name, value, sizeof(value)), _ret == 1);
	CHECK_WITH(value[0], _ret == 'y');
}

static void verify_materials(void)
{
	struct stat st;

	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C01), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C02), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C03), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s03", X_C03), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C04), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s04", X_C04), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C07), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C08), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s08", X_C08), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C10), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s10", X_C10), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C11), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C13), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C16), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s16", X_C16), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C17), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C17), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C18), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C18), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C19), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C19), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C20), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s20", X_C20), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C20), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C21), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s21", X_C21), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C21), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C22), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s22", X_C22), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C22), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C23), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C23), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C24), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C24), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C25), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C25), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/t25", X_C25), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C27), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s27", X_C27), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C27), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C28), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s28", X_C28), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", T_C28), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/t28", X_C28B), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C29), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C30), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C31), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s31", X_C31), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_C32), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/s32", X_C32), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_X01), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/xs01", X_X01), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_X03), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu", S_X12), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pu/xs12", X_X12), _ret == 1);
	CHECK_WITH(count_entries(UPPER_DIR "/pu"), _ret == 43);

	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_C05), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_C06), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_C09), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm/s09", X_C09), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_C12), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_C14), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm/s14", X_C14), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", T_C14), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_C15), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm/s15", X_C15), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", T_C15), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", T_C16), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_C26), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm/s26", X_C26), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", T_C26), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_X02), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_X04), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm/xs04", X_X04A), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_X05), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm/xs05", X_X05A), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_X07), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_X08), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_X09), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm/xs09", X_X09A), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", T_X09), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_X10), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm", S_X11), _ret == 1);
	CHECK_WITH(find_entry(UPPER_DIR "/pm/xs11", X_X11), _ret == 1);
	CHECK_WITH(count_entries(UPPER_DIR "/pm"), _ret == 22);

	CHECK_WITH(count_entries(UPPER_DIR "/pe"), _ret == 0);
	CHECK_WITH(count_entries(UPPER_DIR "/pv"), _ret == 0);
	CHECK_WITH(lstat(UPPER_DIR "/pl", &st), _ret == -1 && errno == ENOENT);

	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_C12), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_C13), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_C14), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_C15), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_C16), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", DEC_A), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_X09), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm/xt09", X_X09B), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", S_X04), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm/xs04", X_X04B), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", S_X05), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm/xs05", X_X05B), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_C05), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_C06), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_C09), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", DEC_B), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_X07), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", S_X02), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", S_X07), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", S_X10), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", S_X11), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", S_X13), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pm", T_X01), _ret == 1);
	CHECK_WITH(count_entries(LOWER_DIR "/pm"), _ret == 21);

	CHECK_WITH(find_entry(LOWER_DIR "/pe", T_C07), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pe", T_C10), _ret == 1);
	CHECK_WITH(count_entries(LOWER_DIR "/pe"), _ret == 2);

	CHECK_WITH(find_entry(LOWER_DIR "/pl", T_C08), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pl", T_C11), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pl", T_X12), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pl/xt12", X_X12B), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pl", S_X06), _ret == 1);
	CHECK_WITH(find_entry(LOWER_DIR "/pl/xs06", X_X06), _ret == 1);
	CHECK_WITH(count_entries(LOWER_DIR "/pl"), _ret == 4);

	verify_whiteout(UPPER_DIR "/pu/t29");
	verify_whiteout(UPPER_DIR "/pu/t30");
	verify_whiteout(UPPER_DIR "/pu/t31");
	verify_whiteout(UPPER_DIR "/pu/t32");
	verify_whiteout(LOWER_DIR "/pm/xt01");
	verify_marker(UPPER_DIR "/pm/xs04", OPAQUE_XATTR);
	verify_marker(UPPER_DIR "/pm/xt09", OPAQUE_XATTR);
	verify_marker(UPPER_DIR "/pv", IMPURE_XATTR);
	CHECK_WITH(lstat(UPPER_DIR "/pu/xs03", &st), _ret == 0);
	CHECK_WITH(S_ISLNK(st.st_mode), _ret);
}

static void build_materials(void)
{
	make_dir(BASE_DIR);
	make_dir(UPPER_DIR);
	make_dir(WORK_DIR);
	make_dir(LOWER_DIR);
	make_dir(MERGED_DIR);

	build_lower_tree();
	build_upper_tree();
	verify_materials();
}

static void mount_overlay(void)
{
	char options[256];

	snprintf(options, sizeof(options), "lowerdir=%s,upperdir=%s,workdir=%s",
		 LOWER_DIR, UPPER_DIR, WORK_DIR);

	CHECK(mount("overlay", MERGED_DIR, "overlay", 0, options));
}

FN_SETUP(init)
{
	build_materials();
	mount_overlay();
}

END_SETUP()

FN_TEST(c01_nd_rp_absent_pu)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C01);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C01);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RU, PU_DIR, T_C01);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src01"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C01);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C01);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c02_nd_nr_absent_pu)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C02);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C02);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RU, PU_DIR, T_C02);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src02"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C02);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C02);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(x01_nd_rp_absent_lowerwhite)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_X01);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X01);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RL, PM_DIR, T_X01);
	verify_whiteout(path);
	physical_path(path, sizeof(path), RU, PM_DIR, T_X01);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_X01), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, T_X01);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
	physical_path(path, sizeof(path), RU, PU_DIR, S_X01);
	TEST_ERRNO(lstat(path, &st), ENOENT);
	physical_path(path, sizeof(path), RL, PM_DIR, T_X01);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
}

END_TEST()

FN_TEST(c03_d_rp_absent_pu)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C03);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C03);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RU, PU_DIR, T_C03);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C03), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C03);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
	physical_path(path, sizeof(path), RU, PU_DIR, S_C03);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c04_d_nr_absent_pu)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C04);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C04);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RU, PU_DIR, T_C04);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C04), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C04);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
	physical_path(path, sizeof(path), RU, PU_DIR, S_C04);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(x02_d_rp_barrier)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X02);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X02);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RU, PM_DIR, S_X02);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RL, PM_DIR, S_X02);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "up02"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, S_X02);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, T_X02);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RL, PM_DIR, S_X02);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
}

END_TEST()

FN_TEST(x03_nd_rp_symlink)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_X03);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_X03);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISLNK(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISLNK(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_RES(readlink(dst, buf, sizeof(buf) - 1), _ret == 14);
	TEST_RES(strcmp(buf, SYMLINK_TARGET), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_X03);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISLNK(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_X03);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(x04_d_rp_barrier_opaque_source)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X04);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X04);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	verify_marker(UPPER_DIR "/pm/xs04", OPAQUE_XATTR);
	physical_path(path, sizeof(path), RL, PM_DIR, S_X04);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_X04A), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, S_X04);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, T_X04);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	physical_path(path, sizeof(path), RL, PM_DIR, S_X04);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
}

END_TEST()

FN_TEST(x05_d_rp_merged_exdev)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X05);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X05);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	CHECK_WITH(find_entry(src, X_X05A), _ret == 1);
	CHECK_WITH(find_entry(src, X_X05B), _ret == 1);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE),
		   EXDEV);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_X05A), _ret == 1);
	TEST_RES(find_entry(src, X_X05B), _ret == 1);
	TEST_ERRNO(lstat(dst, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, S_X05);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(path, X_X05A), _ret == 1);
	physical_path(path, sizeof(path), RU, PM_DIR, T_X05);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(x06_d_rp_merged_exdev_loweronly)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PL_DIR, S_X06);
	merge_path(dst, sizeof(dst), RM PL_DIR, T_X06);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	CHECK_WITH(lstat(UPPER_DIR "/pl", &st), _ret == -1 && errno == ENOENT);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE),
		   EXDEV);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_ERRNO(lstat(dst, &st), ENOENT);
	TEST_ERRNO(lstat(UPPER_DIR "/pl", &st), ENOENT);

	physical_path(path, sizeof(path), RU, PL_DIR, T_X06);
	TEST_ERRNO(lstat(path, &st), ENOENT);
	physical_path(path, sizeof(path), RU, PL_DIR, S_X06);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c05_nd_rp_lowerfile_pm)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_C05);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C05);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_C05);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src05"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, T_C05);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	read_file_body(path, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src05"), _ret == 0);
	physical_path(path, sizeof(path), RL, PM_DIR, T_C05);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, S_C05);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c06_nd_nr_lowerfile_pm)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_C06);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C06);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RL, PM_DIR, T_C06);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_C06);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE),
		   EEXIST);

	read_file_body(src, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src06"), _ret == 0);
	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "low06"), _ret == 0);

	physical_path(path, sizeof(path), RU, PM_DIR, T_C06);
	TEST_ERRNO(lstat(path, &st), ENOENT);
	physical_path(path, sizeof(path), RU, PM_DIR, S_C06);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
}

END_TEST()

FN_TEST(c07_nd_ex_lowerfile_cross)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C07);
	merge_path(dst, sizeof(dst), RM PE_DIR, T_C07);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PE_DIR, T_C07);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(UPPER_DIR "/pe", IMPURE_XATTR, buf, sizeof(buf)),
		   ENODATA);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_EXCHANGE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src07"), _ret == 0);
	read_file_body(src, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "low07"), _ret == 0);

	physical_path(path, sizeof(path), RU, PE_DIR, T_C07);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C07);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	read_file_body(path, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "low07"), _ret == 0);
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pe", IMPURE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	physical_path(path, sizeof(path), RL, PE_DIR, T_C07);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C07);
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(path, ORIGIN_XATTR, buf, sizeof(buf)), _ret >= 32);
}

END_TEST()

FN_TEST(c08_d_rp_lowerfile)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C08);
	merge_path(dst, sizeof(dst), RM PL_DIR, T_C08);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PL_DIR, T_C08);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE),
		   ENOTDIR);

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_C08), _ret == 1);
	TEST_ERRNO(lstat(UPPER_DIR "/pl", &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, S_C08);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PL_DIR, T_C08);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c09_d_nr_lowerfile)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_C09);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C09);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE),
		   EEXIST);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_C09), _ret == 1);
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));

	physical_path(path, sizeof(path), RU, PM_DIR, S_C09);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_C09);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c10_d_ex_lowerfile_cross)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C10);
	merge_path(dst, sizeof(dst), RM PE_DIR, T_C10);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PE_DIR, T_C10);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_EXCHANGE));

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C10), _ret == 1);
	read_file_body(src, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "low10"), _ret == 0);

	physical_path(path, sizeof(path), RU, PE_DIR, T_C10);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C10);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pe", IMPURE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
}

END_TEST()

FN_TEST(c11_nd_rp_lowerdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C11);
	merge_path(dst, sizeof(dst), RM PL_DIR, T_C11);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PL_DIR, T_C11);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE),
		   EISDIR);

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	TEST_ERRNO(lstat(UPPER_DIR "/pl", &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, S_C11);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PL_DIR, T_C11);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c12_nd_nr_lowerdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_C12);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C12);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE),
		   EEXIST);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	physical_path(path, sizeof(path), RU, PM_DIR, S_C12);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_C12);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c13_nd_ex_lowerdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C13);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C13);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_C13);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_EXCHANGE),
		   EXDEV);

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));

	physical_path(path, sizeof(path), RU, PM_DIR, T_C13);
	TEST_ERRNO(lstat(path, &st), ENOENT);
	physical_path(path, sizeof(path), RU, PU_DIR, S_C13);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
}

END_TEST()

FN_TEST(c14_d_rp_mergeddir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_C14);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C14);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_C14);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RL, PM_DIR, T_C14);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_C14);
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C14), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, T_C14);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	physical_path(path, sizeof(path), RU, PM_DIR, S_C14);
	TEST_ERRNO(lstat(path, &st), ENOENT);
	physical_path(path, sizeof(path), RL, PM_DIR, T_C14);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(count_entries(path), _ret == 0);
}

END_TEST()

FN_TEST(c15_d_nr_mergeddir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_C15);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C15);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(UPPER_DIR "/pm/t15", &st), _ret == 0);
	CHECK_WITH(lstat(LOWER_DIR "/pm/t15", &st), _ret == 0);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE),
		   EEXIST);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	physical_path(path, sizeof(path), RU, PM_DIR, T_C15);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
	physical_path(path, sizeof(path), RU, PM_DIR, S_C15);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
}

END_TEST()

FN_TEST(c16_d_ex_mergeddir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C16);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C16);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(UPPER_DIR "/pm/t16", &st), _ret == 0);
	CHECK_WITH(lstat(LOWER_DIR "/pm/t16", &st), _ret == 0);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_EXCHANGE),
		   EXDEV);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_C16), _ret == 1);
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	physical_path(path, sizeof(path), RU, PU_DIR, S_C16);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
	physical_path(path, sizeof(path), RU, PM_DIR, T_C16);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
}

END_TEST()

FN_TEST(x12_d_rp_mergeddir_notempty)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_X12);
	merge_path(dst, sizeof(dst), RM PL_DIR, T_X12);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(find_entry(dst, X_X12B), _ret == 1);
	CHECK_WITH(lstat(UPPER_DIR "/pl", &st), _ret == -1 && errno == ENOENT);

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE),
		   ENOTEMPTY);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_X12), _ret == 1);
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_ERRNO(lstat(UPPER_DIR "/pl", &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, S_X12);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PL_DIR, T_X12);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c17_nd_rp_upperfile)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C17);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C17);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, T_C17);
	read_file_body(path, buf, sizeof(buf));
	CHECK_WITH(strcmp(buf, "old17"), _ret == 0);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src17"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C17);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	read_file_body(path, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src17"), _ret == 0);
	physical_path(path, sizeof(path), RU, PU_DIR, S_C17);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c18_nd_nr_upperfile)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C18);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C18);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE),
		   EEXIST);

	read_file_body(src, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src18"), _ret == 0);
	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "old18"), _ret == 0);

	physical_path(path, sizeof(path), RU, PU_DIR, S_C18);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, T_C18);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
}

END_TEST()

FN_TEST(c19_nd_ex_upperfile)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;
	ino_t src_ino, dst_ino;

	merge_path(src, sizeof(src), RM PU_DIR, S_C19);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C19);
	CHECK_WITH(stat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	src_ino = st.st_ino;
	CHECK_WITH(stat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	dst_ino = st.st_ino;

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_EXCHANGE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src19"), _ret == 0);
	read_file_body(src, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "old19"), _ret == 0);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C19);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	read_file_body(path, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src19"), _ret == 0);
	physical_path(path, sizeof(path), RU, PU_DIR, S_C19);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	read_file_body(path, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "old19"), _ret == 0);
	TEST_RES(stat(src, &st), _ret == 0 && st.st_ino == dst_ino);
	TEST_RES(stat(dst, &st), _ret == 0 && st.st_ino == src_ino);
}

END_TEST()

FN_TEST(x08_nd_rp_upperfile_barrier)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X08);
	merge_path(dst, sizeof(dst), RM PV_DIR, T_X08);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	verify_marker(UPPER_DIR "/pv", IMPURE_XATTR);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src08"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, S_X08);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PV_DIR, T_X08);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pv", IMPURE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, ORIGIN_XATTR, buf, sizeof(buf)), ENODATA);
}

END_TEST()

FN_TEST(c20_d_rp_upperfile)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C20);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C20);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE),
		   ENOTDIR);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_C20), _ret == 1);
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));

	physical_path(path, sizeof(path), RU, PU_DIR, S_C20);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, T_C20);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
}

END_TEST()

FN_TEST(c21_d_nr_upperfile)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C21);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C21);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE),
		   EEXIST);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_C21), _ret == 1);
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));

	physical_path(path, sizeof(path), RU, PU_DIR, S_C21);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, T_C21);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
}

END_TEST()

FN_TEST(c22_d_ex_upperfile)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;
	ino_t src_ino, dst_ino;

	merge_path(src, sizeof(src), RM PU_DIR, S_C22);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C22);
	CHECK_WITH(stat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	src_ino = st.st_ino;
	CHECK_WITH(stat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	dst_ino = st.st_ino;

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_EXCHANGE));

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C22), _ret == 1);
	TEST_RES(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));

	physical_path(path, sizeof(path), RU, PU_DIR, T_C22);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C22);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	TEST_RES(stat(src, &st), _ret == 0 && st.st_ino == dst_ino);
	TEST_RES(stat(dst, &st), _ret == 0 && st.st_ino == src_ino);
}

END_TEST()

FN_TEST(c23_nd_rp_upperdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C23);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C23);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE),
		   EISDIR);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	physical_path(path, sizeof(path), RU, PU_DIR, S_C23);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, T_C23);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
}

END_TEST()

FN_TEST(c24_nd_nr_upperdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C24);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C24);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE),
		   EEXIST);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	physical_path(path, sizeof(path), RU, PU_DIR, S_C24);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, T_C24);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
}

END_TEST()

FN_TEST(c25_nd_ex_upperdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;
	ino_t src_ino, dst_ino;

	merge_path(src, sizeof(src), RM PU_DIR, S_C25);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C25);
	CHECK_WITH(stat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	src_ino = st.st_ino;
	CHECK_WITH(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	dst_ino = st.st_ino;

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_EXCHANGE));

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_C25), _ret == 1);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C25);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C25);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(stat(src, &st), _ret == 0 && st.st_ino == dst_ino);
	TEST_RES(stat(dst, &st), _ret == 0 && st.st_ino == src_ino);
}

END_TEST()

FN_TEST(c26_d_rp_upperdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_C26);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_C26);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RL, PM_DIR, T_C26);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);
	CHECK_WITH(count_entries(LOWER_DIR "/pm"), _ret == 21);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C26), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, T_C26);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
	physical_path(path, sizeof(path), RU, PM_DIR, S_C26);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c27_d_nr_upperdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C27);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C27);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	TEST_ERRNO(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE),
		   EEXIST);

	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_C27), _ret == 1);
	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));

	physical_path(path, sizeof(path), RU, PU_DIR, S_C27);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, T_C27);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
}

END_TEST()

FN_TEST(c28_d_ex_upperdir)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;
	ino_t src_ino, dst_ino;

	merge_path(src, sizeof(src), RM PU_DIR, S_C28);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C28);
	CHECK_WITH(stat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	src_ino = st.st_ino;
	CHECK_WITH(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	dst_ino = st.st_ino;

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_EXCHANGE));

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C28), _ret == 1);
	TEST_RES(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(src, X_C28B), _ret == 1);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C28);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
	physical_path(path, sizeof(path), RU, PU_DIR, S_C28);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), ENODATA);
	TEST_RES(stat(src, &st), _ret == 0 && st.st_ino == dst_ino);
	TEST_RES(stat(dst, &st), _ret == 0 && st.st_ino == src_ino);
}

END_TEST()

FN_TEST(x09_d_rp_upperdir_opaque)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X09);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X09);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(LOWER_DIR "/pm/xt09", &st), _ret == 0);
	verify_marker(UPPER_DIR "/pm/xt09", OPAQUE_XATTR);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(stat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_X09A), _ret == 1);
	TEST_RES(find_entry(dst, X_X09B), _ret == 0);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, T_X09);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(path, X_X09A), _ret == 1);
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(path, OPAQUE_XATTR, buf, sizeof(buf)), _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	physical_path(path, sizeof(path), RL, PM_DIR, T_X09);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(path, X_X09B), _ret == 1);
}

END_TEST()

FN_TEST(c29_nd_rp_whiteout)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C29);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C29);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	verify_whiteout(UPPER_DIR "/pu/t29");

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src29"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C29);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C29);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c30_nd_nr_whiteout)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C30);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C30);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	verify_whiteout(UPPER_DIR "/pu/t30");

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src30"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C30);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C30);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(x10_nd_rp_whiteout_barrier)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X10);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X10);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RL, PM_DIR, S_X10);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	verify_whiteout(UPPER_DIR "/pm/xt10");

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src10"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, S_X10);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, T_X10);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RL, PM_DIR, S_X10);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
}

END_TEST()

FN_TEST(c31_d_rp_whiteout_nobarrier)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C31);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C31);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	verify_whiteout(UPPER_DIR "/pu/t31");

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C31), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C31);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C31);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(c32_d_nr_whiteout)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PU_DIR, S_C32);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_C32);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	verify_whiteout(UPPER_DIR "/pu/t32");

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_NOREPLACE));

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_C32), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PU_DIR, T_C32);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RU, PU_DIR, S_C32);
	TEST_ERRNO(lstat(path, &st), ENOENT);
}

END_TEST()

FN_TEST(x11_d_rp_whiteout_barrier)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X11);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X11);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISDIR(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RL, PM_DIR, S_X11);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	verify_whiteout(UPPER_DIR "/pm/xt11");

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	TEST_RES(lstat(dst, &st), _ret == 0 && S_ISDIR(st.st_mode));
	TEST_RES(find_entry(dst, X_X11), _ret == 1);
	TEST_RES(count_entries(dst), _ret == 1);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, S_X11);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, T_X11);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISDIR(st.st_mode));
	physical_path(path, sizeof(path), RL, PM_DIR, S_X11);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
}

END_TEST()

FN_TEST(x07_nd_rp_lowerfile_barrier)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X07);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X07);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RL, PM_DIR, S_X07);
	CHECK_WITH(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_X07);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "src07x"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);

	physical_path(path, sizeof(path), RU, PM_DIR, T_X07);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, S_X07);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
}

END_TEST()

FN_TEST(x13_impure_origin_chain)
{
	char src[PATH_MAX], dst[PATH_MAX], path[PATH_MAX];
	char buf[64];
	struct stat st;

	merge_path(src, sizeof(src), RM PM_DIR, S_X13);
	merge_path(dst, sizeof(dst), RM PM_DIR, T_X13);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	physical_path(path, sizeof(path), RU, PM_DIR, S_X13);
	CHECK_WITH(lstat(path, &st), _ret == -1 && errno == ENOENT);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "low13"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);
	physical_path(path, sizeof(path), RU, PM_DIR, S_X13);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, T_X13);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(path, ORIGIN_XATTR, buf, sizeof(buf)), _ret >= 32);

	merge_path(src, sizeof(src), RM PM_DIR, T_X13);
	merge_path(dst, sizeof(dst), RM PU_DIR, T_X13B);
	CHECK_WITH(lstat(src, &st), _ret == 0 && S_ISREG(st.st_mode));
	CHECK_WITH(lstat(dst, &st), _ret == -1 && errno == ENOENT);
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(UPPER_DIR "/pu", IMPURE_XATTR, buf, sizeof(buf)),
		   ENODATA);

	TEST_SUCC(renameat2(AT_FDCWD, src, AT_FDCWD, dst, MODE_REPLACE));

	read_file_body(dst, buf, sizeof(buf));
	TEST_RES(strcmp(buf, "low13"), _ret == 0);
	TEST_ERRNO(lstat(src, &st), ENOENT);
	physical_path(path, sizeof(path), RU, PU_DIR, T_X13B);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISREG(st.st_mode));
	physical_path(path, sizeof(path), RU, PM_DIR, T_X13);
	TEST_ERRNO(lstat(path, &st), ENOENT);
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pu", IMPURE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
}

END_TEST()

FN_TEST(final_state_cross_check)
{
	char path[PATH_MAX];
	char buf[64];
	struct stat st;

	TEST_RES(count_entries(UPPER_DIR "/pu"), _ret == 38);
	TEST_RES(count_entries(UPPER_DIR "/pm"), _ret == 24);
	TEST_RES(count_entries(UPPER_DIR "/pe"), _ret == 2);
	TEST_RES(count_entries(UPPER_DIR "/pv"), _ret == 1);
	TEST_ERRNO(lstat(UPPER_DIR "/pl", &st), ENOENT);
	TEST_RES(count_entries(LOWER_DIR "/pm"), _ret == 21);
	TEST_RES(count_entries(LOWER_DIR "/pe"), _ret == 2);
	TEST_RES(count_entries(LOWER_DIR "/pl"), _ret == 4);

	TEST_RES(count_char_dev_zero(UPPER_DIR "/pu"), _ret == 0);
	TEST_RES(count_char_dev_zero(UPPER_DIR "/pm"), _ret == 7);
	TEST_RES(count_char_dev_zero(UPPER_DIR "/pe"), _ret == 0);
	TEST_RES(count_char_dev_zero(UPPER_DIR "/pv"), _ret == 0);

	TEST_RES(find_entry(UPPER_DIR "/pu", T_C01), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C02), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C03), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C04), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_X03), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_X13B), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C07), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C08), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C10), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C11), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C13), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C16), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C17), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C18), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C18), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C19), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C19), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C20), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C20), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C21), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C21), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C22), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C22), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C23), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C23), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C24), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C24), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C25), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C25), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C27), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C27), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C28), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C28), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C29), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C30), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C31), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", T_C32), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_X12), _ret == 1);

	TEST_RES(find_entry(UPPER_DIR "/pu", S_C01), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C02), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C03), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C04), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C17), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C29), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C30), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C31), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_C32), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_X01), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pu", S_X03), _ret == 0);

	TEST_RES(find_entry(UPPER_DIR "/pm", T_C05), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_C06), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_C09), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_C12), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_C14), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_C15), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_C15), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_C16), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_C26), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_X01), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_X02), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_X04), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_X07), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_X09), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X02), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X04), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X05), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X07), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X08), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X10), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_X10), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X11), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_X11), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X13), _ret == 1);

	TEST_RES(find_entry(UPPER_DIR "/pm", S_C05), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_C14), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_C26), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pm", S_X09), _ret == 0);
	TEST_RES(find_entry(UPPER_DIR "/pm", T_X13), _ret == 0);

	TEST_RES(find_entry(UPPER_DIR "/pe", T_C07), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pe", T_C10), _ret == 1);
	TEST_RES(find_entry(UPPER_DIR "/pv", T_X08), _ret == 1);

	physical_path(path, sizeof(path), RU, PM_DIR, S_X02);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, S_X04);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, S_X07);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, S_X10);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, S_X11);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);
	physical_path(path, sizeof(path), RU, PM_DIR, S_X13);
	TEST_RES(lstat(path, &st), _ret == 0 && S_ISCHR(st.st_mode));
	TEST_RES(st.st_rdev, _ret == 0);

	TEST_RES(count_opaque_dirs(UPPER_DIR "/pm"), _ret == 3);
	TEST_RES(count_opaque_dirs(UPPER_DIR "/pu"), _ret == 0);

	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pm/t14", OPAQUE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pm/xt04", OPAQUE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pm/xt09", OPAQUE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(UPPER_DIR "/pm/t15", OPAQUE_XATTR, buf,
			    sizeof(buf)),
		   ENODATA);
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(UPPER_DIR "/pm/t16", OPAQUE_XATTR, buf,
			    sizeof(buf)),
		   ENODATA);
	memset(buf, 0, sizeof(buf));
	TEST_ERRNO(getxattr(UPPER_DIR "/pm/t26", OPAQUE_XATTR, buf,
			    sizeof(buf)),
		   ENODATA);

	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pm", IMPURE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pu", IMPURE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pe", IMPURE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');
	memset(buf, 0, sizeof(buf));
	TEST_RES(getxattr(UPPER_DIR "/pv", IMPURE_XATTR, buf, sizeof(buf)),
		 _ret == 1);
	TEST_RES(buf[0], _ret == 'y');

	TEST_RES(find_entry(LOWER_DIR "/pm", T_C05), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_C06), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_C09), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_C12), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_C13), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_C14), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_C15), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_C16), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_X01), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_X07), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", T_X09), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", S_X02), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", S_X04), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", S_X05), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", S_X07), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", S_X10), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", S_X11), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", S_X13), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", DEC_A), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pm", DEC_B), _ret == 1);

	TEST_RES(find_entry(LOWER_DIR "/pe", T_C07), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pe", T_C10), _ret == 1);

	TEST_RES(find_entry(LOWER_DIR "/pl", T_C08), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pl", T_C11), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pl", T_X12), _ret == 1);
	TEST_RES(find_entry(LOWER_DIR "/pl", S_X06), _ret == 1);
}

END_TEST()

FN_SETUP(cleanup)
{
	CHECK(umount(MERGED_DIR));

	remove_tree(UPPER_DIR "/pu/s01");
	remove_tree(UPPER_DIR "/pu/s02");
	remove_tree(UPPER_DIR "/pu/s03");
	remove_tree(UPPER_DIR "/pu/s04");
	remove_tree(UPPER_DIR "/pu/s07");
	remove_tree(UPPER_DIR "/pu/s08");
	remove_tree(UPPER_DIR "/pu/s10");
	remove_tree(UPPER_DIR "/pu/s11");
	remove_tree(UPPER_DIR "/pu/s13");
	remove_tree(UPPER_DIR "/pu/s16");
	remove_tree(UPPER_DIR "/pu/s17");
	remove_tree(UPPER_DIR "/pu/t17");
	remove_tree(UPPER_DIR "/pu/s18");
	remove_tree(UPPER_DIR "/pu/t18");
	remove_tree(UPPER_DIR "/pu/s19");
	remove_tree(UPPER_DIR "/pu/t19");
	remove_tree(UPPER_DIR "/pu/s20");
	remove_tree(UPPER_DIR "/pu/t20");
	remove_tree(UPPER_DIR "/pu/s21");
	remove_tree(UPPER_DIR "/pu/t21");
	remove_tree(UPPER_DIR "/pu/s22");
	remove_tree(UPPER_DIR "/pu/t22");
	remove_tree(UPPER_DIR "/pu/s23");
	remove_tree(UPPER_DIR "/pu/t23");
	remove_tree(UPPER_DIR "/pu/s24");
	remove_tree(UPPER_DIR "/pu/t24");
	remove_tree(UPPER_DIR "/pu/s25");
	remove_tree(UPPER_DIR "/pu/t25");
	remove_tree(UPPER_DIR "/pu/s27");
	remove_tree(UPPER_DIR "/pu/t27");
	remove_tree(UPPER_DIR "/pu/s28");
	remove_tree(UPPER_DIR "/pu/t28");
	remove_tree(UPPER_DIR "/pu/s29");
	remove_tree(UPPER_DIR "/pu/s30");
	remove_tree(UPPER_DIR "/pu/s31");
	remove_tree(UPPER_DIR "/pu/s32");
	remove_tree(UPPER_DIR "/pu/t01");
	remove_tree(UPPER_DIR "/pu/t02");
	remove_tree(UPPER_DIR "/pu/t03");
	remove_tree(UPPER_DIR "/pu/t04");
	remove_tree(UPPER_DIR "/pu/xt03");
	remove_tree(UPPER_DIR "/pu/xt13b");
	remove_tree(UPPER_DIR "/pu/t29");
	remove_tree(UPPER_DIR "/pu/t30");
	remove_tree(UPPER_DIR "/pu/t31");
	remove_tree(UPPER_DIR "/pu/t32");
	remove_tree(UPPER_DIR "/pu/xs01");
	remove_tree(UPPER_DIR "/pu/xs03");
	remove_tree(UPPER_DIR "/pu/xs12");

	remove_tree(UPPER_DIR "/pm/s05");
	remove_tree(UPPER_DIR "/pm/s06");
	remove_tree(UPPER_DIR "/pm/s09");
	remove_tree(UPPER_DIR "/pm/s12");
	remove_tree(UPPER_DIR "/pm/s14");
	remove_tree(UPPER_DIR "/pm/s15");
	remove_tree(UPPER_DIR "/pm/s26");
	remove_tree(UPPER_DIR "/pm/t05");
	remove_tree(UPPER_DIR "/pm/t14");
	remove_tree(UPPER_DIR "/pm/t15");
	remove_tree(UPPER_DIR "/pm/t16");
	remove_tree(UPPER_DIR "/pm/t26");
	remove_tree(UPPER_DIR "/pm/xt01");
	remove_tree(UPPER_DIR "/pm/xt02");
	remove_tree(UPPER_DIR "/pm/xt04");
	remove_tree(UPPER_DIR "/pm/xt07");
	remove_tree(UPPER_DIR "/pm/xt09");
	remove_tree(UPPER_DIR "/pm/xt10");
	remove_tree(UPPER_DIR "/pm/xt11");
	remove_tree(UPPER_DIR "/pm/xt13");
	remove_tree(UPPER_DIR "/pm/xs02");
	remove_tree(UPPER_DIR "/pm/xs04");
	remove_tree(UPPER_DIR "/pm/xs05");
	remove_tree(UPPER_DIR "/pm/xs07");
	remove_tree(UPPER_DIR "/pm/xs08");
	remove_tree(UPPER_DIR "/pm/xs09");
	remove_tree(UPPER_DIR "/pm/xs10");
	remove_tree(UPPER_DIR "/pm/xs11");
	remove_tree(UPPER_DIR "/pm/xs13");

	remove_tree(UPPER_DIR "/pe/t07");
	remove_tree(UPPER_DIR "/pe/t10");
	remove_tree(UPPER_DIR "/pv/xt08");

	remove_tree(LOWER_DIR "/pm/t05");
	remove_tree(LOWER_DIR "/pm/t06");
	remove_tree(LOWER_DIR "/pm/t09");
	remove_tree(LOWER_DIR "/pm/t12");
	remove_tree(LOWER_DIR "/pm/t13");
	remove_tree(LOWER_DIR "/pm/t14");
	remove_tree(LOWER_DIR "/pm/t15");
	remove_tree(LOWER_DIR "/pm/t16");
	remove_tree(LOWER_DIR "/pm/" DEC_A);
	remove_tree(LOWER_DIR "/pm/" DEC_B);
	remove_tree(LOWER_DIR "/pm/xt01");
	remove_tree(LOWER_DIR "/pm/xt07");
	remove_tree(LOWER_DIR "/pm/xt09");
	remove_tree(LOWER_DIR "/pm/xs02");
	remove_tree(LOWER_DIR "/pm/xs04");
	remove_tree(LOWER_DIR "/pm/xs05");
	remove_tree(LOWER_DIR "/pm/xs07");
	remove_tree(LOWER_DIR "/pm/xs10");
	remove_tree(LOWER_DIR "/pm/xs11");
	remove_tree(LOWER_DIR "/pm/xs13");

	remove_tree(LOWER_DIR "/pe/t07");
	remove_tree(LOWER_DIR "/pe/t10");

	remove_tree(LOWER_DIR "/pl/t08");
	remove_tree(LOWER_DIR "/pl/t11");
	remove_tree(LOWER_DIR "/pl/xt12");
	remove_tree(LOWER_DIR "/pl/xs06");

	CHECK(rmdir(MERGED_DIR));
	remove_tree(WORK_DIR "/work");
	CHECK(rmdir(WORK_DIR));
	remove_tree(UPPER_DIR PU_DIR);
	remove_tree(UPPER_DIR PM_DIR);
	remove_tree(UPPER_DIR PE_DIR);
	remove_tree(UPPER_DIR PV_DIR);
	CHECK(rmdir(UPPER_DIR));
	remove_tree(LOWER_DIR "/pm");
	remove_tree(LOWER_DIR "/pe");
	remove_tree(LOWER_DIR "/pl");
	CHECK(rmdir(LOWER_DIR));
	CHECK(rmdir(BASE_DIR));
}

END_SETUP()
