#ifndef _TWOYI_UTILS_STRING8_H
#define _TWOYI_UTILS_STRING8_H
/*
 * Compatibility shim for <utils/String8.h>. Wraps std::string to
 * provide the android::String8 API surface used by the emugl sources.
 */
#include <string>
#include <cstring>
#include <cstddef>

namespace android {

class String8 : public std::string {
public:
    String8() : std::string() {}
    String8(const char *s) : std::string(s ? s : "") {}
    String8(const char *s, size_t len) : std::string(s, len) {}
    String8(const std::string &s) : std::string(s) {}

    const char *string() const { return c_str(); }

    String8 &append(const char *s) { std::string::append(s); return *this; }
    String8 &append(const char *s, size_t len) { std::string::append(s, len); return *this; }
    String8 &append(const String8 &s) { std::string::append(s); return *this; }

    size_t length() const { return std::string::size(); }

    /* implicit conversion to const char* used in some emugl call sites */
    operator const char *() const { return c_str(); }

    bool operator==(const char *rhs) const { return compare(rhs ? rhs : "") == 0; }
    bool operator<(const String8 &rhs) const { return compare(rhs) < 0; }
    bool operator<(const char *rhs) const { return compare(rhs ? rhs : "") < 0; }
};

}  // namespace android

#endif /* _TWOYI_UTILS_STRING8_H */
