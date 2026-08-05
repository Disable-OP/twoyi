#ifndef _TWOYI_UTILS_LIST_H
#define _TWOYI_UTILS_LIST_H
/*
 * Compatibility shim for <utils/List.h>.
 * Wraps std::list to provide the android::List<T> API surface.
 */
#include <list>

namespace android {

template <class T>
class List : public std::list<T> {
public:
    typedef typename std::list<T>::iterator iterator;
    typedef typename std::list<T>::const_iterator const_iterator;
    typedef typename std::list<T>::reverse_iterator reverse_iterator;

    void pushFront(const T &v) { std::list<T>::push_front(v); }
    void pushBack(const T &v)  { std::list<T>::push_back(v); }

    void insert(iterator pos, const T &v) { std::list<T>::insert(pos, v); }
    iterator erase(iterator pos) { return std::list<T>::erase(pos); }
};

}  // namespace android

#endif /* _TWOYI_UTILS_LIST_H */
