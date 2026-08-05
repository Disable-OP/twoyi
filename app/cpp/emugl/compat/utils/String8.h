#pragma once
#include <string>
namespace android {
class String8 : public std::string {
public:
    String8() {}
    String8(const char* s) : std::string(s ? s : '') {}
    String8(const std::string& s) : std::string(s) {}
    const char* string() const { return c_str(); }
};
}
