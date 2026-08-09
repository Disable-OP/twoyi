#pragma once
#include <vector>
#include <stddef.h>
namespace android {

// android::Vector — extends std::vector with the Android-specific
// method names that the emugl sources use (insertAt, removeAt, add).
template <typename T>
class Vector : public std::vector<T> {
public:
    // insertAt — insert `item` at `index`. Matches android::Vector::insertAt.
    ssize_t insertAt(const T& item, size_t index, size_t num = 1) {
        if (index > this->size()) return -1;
        auto it = this->begin() + index;
        this->insert(it, num, item);
        return static_cast<ssize_t>(index);
    }

    // removeAt — remove the element at `index`.
    void removeAt(size_t index) {
        if (index >= this->size()) return;
        this->erase(this->begin() + index);
    }

    // add — append to the end (Android API).
    ssize_t add(const T& item) {
        this->push_back(item);
        return static_cast<ssize_t>(this->size() - 1);
    }
};

}  // namespace android
