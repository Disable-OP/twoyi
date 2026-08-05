#pragma once
#include <list>
namespace android {
template <typename T>
class List : public std::list<T> {};
}
