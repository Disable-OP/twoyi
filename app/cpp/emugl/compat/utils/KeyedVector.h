#ifndef _TWOYI_UTILS_KEYED_VECTOR_H
#define _TWOYI_UTILS_KEYED_VECTOR_H
/*
 * Compatibility shim for <utils/KeyedVector.h> and
 * <utils/DefaultKeyedVector.h>.
 *
 * Backed by std::vector<std::pair<K,V>> (preserves insertion order,
 * which matches the legacy AOSP KeyedVector behaviour for small N) and
 * a std::map for O(log N) key lookup.
 */
#include <vector>
#include <map>
#include <utility>
#include <cstddef>
#include <stdint.h>

/* The AOSP <utils/KeyedVector.h> transitively pulls in <utils/Vector.h>
 * (its SortedVector implementation is built on Vector). Several emugl
 * sources rely on that transitive include, so match it here. */
#include <utils/Vector.h>

namespace android {

template <typename KEY, typename VALUE>
class KeyedVector {
public:
    typedef KEY key_type;
    typedef VALUE value_type;

    KeyedVector() {}

    inline size_t size() const { return mItems.size(); }
    inline bool isEmpty() const { return mItems.empty(); }

    inline const VALUE &valueFor(const KEY &key) const {
        return mItems[indexForKey(key)].second;
    }
    inline VALUE &editValueFor(const KEY &key) {
        return mItems[indexForKey(key)].second;
    }
    inline const VALUE &valueAt(size_t index) const { return mItems[index].second; }
    inline VALUE &editValueAt(size_t index) { return mItems[index].second; }
    inline const KEY &keyAt(size_t index) const { return mItems[index].first; }

    inline ssize_t indexOfKey(const KEY &key) const {
        typename std::map<KEY, size_t>::const_iterator it = mIndex.find(key);
        if (it == mIndex.end()) return -1;
        return (ssize_t)it->second;
    }

    inline ssize_t add(const KEY &key, const VALUE &value) {
        if (mIndex.find(key) != mIndex.end()) {
            return -1;  // already exists
        }
        mIndex[key] = mItems.size();
        mItems.push_back(std::make_pair(key, value));
        return mItems.size() - 1;
    }

    inline ssize_t replaceValueFor(const KEY &key, const VALUE &value) {
        ssize_t i = indexOfKey(key);
        if (i < 0) return i;
        mItems[i].second = value;
        return i;
    }

    inline ssize_t removeItem(const KEY &key) {
        ssize_t i = indexOfKey(key);
        if (i < 0) return i;
        removeItemsAt((size_t)i, 1);
        return i;
    }

    inline void removeItemsAt(size_t index, size_t count = 1) {
        if (index + count > mItems.size()) return;
        mItems.erase(mItems.begin() + index, mItems.begin() + index + count);
        /* rebuild the index */
        mIndex.clear();
        for (size_t k = 0; k < mItems.size(); k++) {
            mIndex[mItems[k].first] = k;
        }
    }

    inline void clear() { mItems.clear(); mIndex.clear(); }

private:
    size_t indexForKey(const KEY &key) const {
        typename std::map<KEY, size_t>::const_iterator it = mIndex.find(key);
        return it->second;
    }

    std::vector<std::pair<KEY, VALUE> > mItems;
    std::map<KEY, size_t>               mIndex;
};

template <typename KEY, typename VALUE>
class DefaultKeyedVector : public KeyedVector<KEY, VALUE> {
public:
    DefaultKeyedVector(const VALUE &defValue = VALUE())
        : mDefault(defValue) {}
    inline const VALUE &valueFor(const KEY &key) const {
        ssize_t i = this->indexOfKey(key);
        return (i < 0) ? mDefault : KeyedVector<KEY, VALUE>::valueAt((size_t)i);
    }
private:
    VALUE mDefault;
};

}  // namespace android

#endif /* _TWOYI_UTILS_KEYED_VECTOR_H */
