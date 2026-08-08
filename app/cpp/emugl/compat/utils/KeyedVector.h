#pragma once
#include <map>
#include <stddef.h>
namespace android {

// android::KeyedVector — read-only map (Android's KeyedVector doesn't
// allow editing via operator[]; you must use add() or editValueFor()).
template <typename K, typename V>
class KeyedVector {
public:
    KeyedVector() {}

    V& editValueFor(const K& key) { return mData[key]; }
    const V& valueFor(const K& key) const {
        auto it = mData.find(key);
        if (it == mData.end()) { static const V def{}; return def; }
        return it->second;
    }

    ssize_t add(const K& key, const V& value) { mData[key] = value; return 0; }
    void removeItem(const K& key) { mData.erase(key); }
    size_t size() const { return mData.size(); }
    bool isEmpty() const { return mData.empty(); }

    // clear — remove all entries.
    void clear() { mData.clear(); }

    // indexOfKey — returns the index of `key` (as an ordering position),
    // or -1 if not found. Android's KeyedVector is backed by a sorted
    // array so indexOfKey is O(log n); our std::map-backed version is
    // also O(log n) via std::distance.
    ssize_t indexOfKey(const K& key) const {
        auto it = mData.find(key);
        if (it == mData.end()) return -1;
        return static_cast<ssize_t>(std::distance(mData.begin(), it));
    }

    // valueAt — returns the value at positional `index`. Requires
    // 0 <= index < size(). Uses std::advance on a non-const iterator
    // (std::map doesn't support random access, so this is O(n)).
    V& valueAt(size_t index) {
        auto it = mData.begin();
        std::advance(it, index);
        return it->second;
    }
    const V& valueAt(size_t index) const {
        auto it = mData.begin();
        std::advance(it, index);
        return it->second;
    }

    // removeItemsAt — remove `count` entries starting at positional
    // `index`. Android's API name; rarely used with count > 1.
    void removeItemsAt(size_t index, size_t count = 1) {
        auto it = mData.begin();
        std::advance(it, index);
        for (size_t i = 0; i < count && it != mData.end(); ++i) {
            it = mData.erase(it);
        }
    }

    // replaceValueFor — replace the value for `key` (insert if absent).
    void replaceValueFor(const K& key, const V& value) { mData[key] = value; }

protected:
    std::map<K, V> mData;
};

// android::DefaultKeyedVector — KeyedVector with a default value
// returned when the key is not found (instead of asserting/crashing
// like the read-only KeyedVector does).
template <typename K, typename V>
class DefaultKeyedVector : public KeyedVector<K, V> {
public:
    DefaultKeyedVector(const V& defVal = V()) : mDefault(defVal) {}

    const V& valueFor(const K& key) const {
        auto it = this->mData.find(key);
        if (it == this->mData.end()) return mDefault;
        return it->second;
    }

private:
    V mDefault;
};

}  // namespace android
