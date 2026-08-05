#ifndef _TWOYI_UTILS_VECTOR_H
#define _TWOYI_UTILS_VECTOR_H
/*
 * Compatibility shim for <utils/Vector.h>.
 * Wraps std::vector to provide the android::Vector<T> API surface
 * used by the emugl sources.
 */
#include <vector>
#include <cstddef>
#include <stdint.h>

namespace android {

template <class TYPE>
class Vector : private std::vector<TYPE> {
public:
    typedef TYPE value_type;

    Vector() {}

    inline size_t size() const { return std::vector<TYPE>::size(); }
    inline bool isEmpty() const { return std::vector<TYPE>::empty(); }
    inline bool empty() const { return std::vector<TYPE>::empty(); }
    inline size_t capacity() const { return std::vector<TYPE>::capacity(); }

    inline const TYPE *array() const { return std::vector<TYPE>::data(); }
    inline TYPE *editArray() { return std::vector<TYPE>::data(); }

    inline const TYPE &operator[](size_t index) const {
        return std::vector<TYPE>::operator[](index);
    }
    inline TYPE &operator[](size_t index) {
        return std::vector<TYPE>::operator[](index);
    }
    inline TYPE &editItemAt(size_t index) {
        return std::vector<TYPE>::operator[](index);
    }
    inline const TYPE &itemAt(size_t index) const {
        return std::vector<TYPE>::operator[](index);
    }
    inline const TYPE &top() const {
        return std::vector<TYPE>::back();
    }

    inline void clear() { std::vector<TYPE>::clear(); }
    inline void removeAt(size_t index) { std::vector<TYPE>::erase(std::vector<TYPE>::begin() + index); }
    inline void removeItemsAt(size_t index, size_t count) {
        std::vector<TYPE>::erase(std::vector<TYPE>::begin() + index,
                                  std::vector<TYPE>::begin() + index + count);
    }

    inline ssize_t add(const TYPE &item) {
        std::vector<TYPE>::push_back(item);
        return std::vector<TYPE>::size() - 1;
    }

    /* AOSP signature: insertAt(item, index, count=1) */
    inline ssize_t insertAt(const TYPE &item, size_t index, size_t count = 1) {
        std::vector<TYPE>::insert(std::vector<TYPE>::begin() + index, count, item);
        return index;
    }
    /* Overload for inserting `count` default-constructed items. */
    inline ssize_t insertAt(size_t index, size_t count = 1) {
        std::vector<TYPE>::insert(std::vector<TYPE>::begin() + index, count, TYPE());
        return index;
    }

    inline ssize_t appendArray(const TYPE *array, size_t count) {
        std::vector<TYPE>::insert(std::vector<TYPE>::end(), array, array + count);
        return std::vector<TYPE>::size() - 1;
    }

    inline void setCapacity(size_t c) { std::vector<TYPE>::reserve(c); }
};

}  // namespace android

#endif /* _TWOYI_UTILS_VECTOR_H */
