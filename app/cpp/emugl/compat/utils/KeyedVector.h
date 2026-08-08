#pragma once
#include <map>
#include <stddef.h>
namespace android {

// android::KeyedVector — read-only map (Android's KeyedVector doesn't
// allow editing via operator[]; you must use add() or editValueFor()).
template <typename K, typename V>
class KeyedVector {
public:
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
