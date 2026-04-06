// SPDX-License-Identifier: GPL-2.0-only
// nuke_ext4_sysfs KPM for APatch/KernelPatch.

#include <linux/errno.h>
#include <linux/fs.h>
#include <linux/kernel.h>
#include <linux/namei.h>
#include <linux/path.h>
#include <linux/printk.h>
#include <linux/string.h>

#include <kallsyms.h>
#include <kpmodule.h>

KPM_NAME("nuke_ext4_sysfs");
KPM_VERSION("0.2.0");
KPM_LICENSE("GPL v2");
KPM_AUTHOR("Hybrid Mount Developers");
KPM_DESCRIPTION("Expose nuke_ext4_sysfs for Hybrid Mount in APatch env");

typedef void (*ext4_unregister_sysfs_t)(struct super_block *sb);

static ext4_unregister_sysfs_t ext4_unregister_sysfs_ptr;

static long resolve_ext4_unregister_sysfs(void) {
    if (ext4_unregister_sysfs_ptr) {
        return 0;
    }

    if (!kallsyms_lookup_name) {
        pr_err("[hm-kpm] kallsyms_lookup_name is unavailable\n");
        return -EOPNOTSUPP;
    }

    ext4_unregister_sysfs_ptr =
        (ext4_unregister_sysfs_t)kallsyms_lookup_name("ext4_unregister_sysfs");
    if (!ext4_unregister_sysfs_ptr) {
        pr_err("[hm-kpm] ext4_unregister_sysfs symbol not found\n");
        return -ENOENT;
    }

    pr_info("[hm-kpm] ext4_unregister_sysfs=%px\n", ext4_unregister_sysfs_ptr);
    return 0;
}

static long do_nuke_ext4_sysfs(const char *path) {
    struct path resolved_path;
    struct super_block *sb;
    char procfs_path[96];
    int err;
    long rc;

    if (!path || !path[0]) {
        return -EINVAL;
    }

    pr_info("[hm-kpm] request: %s\n", path);
    rc = resolve_ext4_unregister_sysfs();
    if (rc) {
        return rc;
    }

    err = kern_path(path, 0, &resolved_path);
    if (err) {
        pr_err("[hm-kpm] kern_path failed: path=%s err=%d\n", path, err);
        return err;
    }

    sb = resolved_path.dentry->d_inode->i_sb;
    if (!sb || !sb->s_type || !sb->s_type->name) {
        pr_err("[hm-kpm] invalid super block for path=%s\n", path);
        path_put(&resolved_path);
        return -EINVAL;
    }

    if (strcmp(sb->s_type->name, "ext4") != 0) {
        pr_err("[hm-kpm] target is not ext4: path=%s fs=%s\n", path,
               sb->s_type->name);
        path_put(&resolved_path);
        return -EOPNOTSUPP;
    }

    snprintf(procfs_path, sizeof(procfs_path), "/proc/fs/ext4/%s", sb->s_id);
    pr_info("[hm-kpm] unregistering ext4 sysfs node: sb=%s proc=%s\n", sb->s_id,
            procfs_path);
    ext4_unregister_sysfs_ptr(sb);
    path_put(&resolved_path);

    err = kern_path(procfs_path, 0, &resolved_path);
    if (!err) {
        pr_err("[hm-kpm] procfs node still present after unregister: %s\n",
               procfs_path);
        path_put(&resolved_path);
        return -EEXIST;
    }
    if (err != -ENOENT) {
        pr_err("[hm-kpm] failed to verify procfs node removal: path=%s err=%d\n",
               procfs_path, err);
        return err;
    }

    pr_info("[hm-kpm] procfs node removed: %s\n", procfs_path);
    return 0;
}

static long hm_control(const char *args, char *out_msg, int outlen) {
    long rc = do_nuke_ext4_sysfs(args);

    if (out_msg && outlen > 0) {
        scnprintf(out_msg, outlen, "rc=%ld", rc);
    }
    return rc;
}

static long hm_control_nr(void *a1, void *a2, void *a3) {
    (void)a2;
    (void)a3;
    return do_nuke_ext4_sysfs((const char *)a1);
}

static long hm_init(const char *args, const char *event, void *reserved) {
    (void)args;
    (void)event;
    (void)reserved;
    pr_info("[hm-kpm] init\n");
    return resolve_ext4_unregister_sysfs();
}

static long hm_exit(void *reserved) {
    (void)reserved;
    pr_info("[hm-kpm] exit\n");
    return 0;
}

KPM_CTL0(hm_control);
KPM_CTL1(hm_control_nr);
KPM_INIT(hm_init);
KPM_EXIT(hm_exit);
