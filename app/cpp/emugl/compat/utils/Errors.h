#ifndef _TWOYI_UTILS_ERRORS_H
#define _TWOYI_UTILS_ERRORS_H
/*
 * Compatibility shim for <utils/Errors.h>.
 */
#include <stdint.h>
#include <sys/types.h>

namespace android {

typedef int32_t status_t;

enum {
    NO_ERROR          = 0,
    UNKNOWN_ERROR     = 0x80000000,
    NO_MEMORY         = -ENOMEM,
    INVALID_OPERATION = -ENOSYS,
    BAD_VALUE         = -EINVAL,
    BAD_TYPE          = 0x80000001,
    NAME_NOT_FOUND    = -ENOENT,
    PERMISSION_DENIED = -EPERM,
    NO_INIT           = -ENODEV,
    ALREADY_EXISTS    = -EEXIST,
    DEAD_OBJECT       = -EPIPE,
    FAILED_TRANSACTION= 0x80000002,
    JPARKS_BROKE_IT   = -EIO,
    BAD_INDEX         = -EOVERFLOW,
    NOT_ENOUGH_DATA   = -ENODATA,
    WOULD_BLOCK       = -EWOULDBLOCK,
    TIMED_OUT         = -ETIMEDOUT,
    UNKNOWN_TRANSACTION = 0x80000003,
};

}  // namespace android

#endif /* _TWOYI_UTILS_ERRORS_H */
