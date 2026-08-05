#pragma once
#include <map>
namespace android {
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
private:
    std::map<K, V> mData;
};
}
