/**
 * @file sample.cpp
 * @brief Comprehensive C++ fixture for testing the rich extractor.
 *
 * This file covers all major C++ constructs: classes (with inheritance,
 * access specifiers, virtual/override/final, constructors, destructors,
 * deleted/defaulted special members), templates, namespaces, concepts,
 * operator overloading, using declarations/aliases, constexpr/consteval/
 * constinit, static_assert, RAII patterns, extern "C" linkage, scoped
 * enums, lambdas, and more.
 */

/* ===== Includes ===== */
#include <iostream>
#include <vector>
#include <string>
#include <memory>
#include <functional>
#include <type_traits>
#include <cstdint>
#include <algorithm>
#include <concepts>

/* ===== Preprocessor Defines ===== */
#define MAX_SIZE 256
#define APP_VERSION "2.0"

/* ===== Forward Declarations ===== */
class Widget;
struct Config;

/* ===== Constexpr Constants ===== */
constexpr int MAX_ELEMENTS = 1024;
constexpr double PI = 3.14159265358979;
constexpr int BUFFER_SIZE = 4096;

/* ===== Constinit Global ===== */
constinit int global_init_val = 42;

/* ===== Scoped Enum ===== */
/// Color enum with RGB values.
enum class Color : uint8_t {
    Red = 0,
    Green = 1,
    Blue = 2
};

/// HTTP status codes.
enum class StatusCode : int {
    OK = 200,
    NotFound = 404,
    InternalError = 500
};

/// Log severity levels (unscoped).
enum LogLevel {
    LOG_DEBUG = 0,
    LOG_INFO = 1,
    LOG_WARN = 2,
    LOG_ERROR = 3
};

/* ===== Using Aliases ===== */
using StringVec = std::vector<std::string>;
using Callback = std::function<void(int)>;
using Size = std::size_t;
using CompareFunc = bool(*)(int, int);

/* ===== Using Declarations ===== */
using std::cout;
using std::endl;

/* ===== Static Assert ===== */
static_assert(sizeof(int) >= 4, "int must be at least 32 bits");
static_assert(sizeof(void*) == 8, "64-bit platform required");
static_assert(MAX_ELEMENTS > 0);

/* ===== Extern "C" Block ===== */
extern "C" {
    void c_init(void);
    int c_process(const char* data, int len);
}

/// Single extern "C" function.
extern "C" void c_cleanup(void);

/* ===== Namespace ===== */

/// Mathematical utilities namespace.
namespace math {

/// Compute the absolute value.
int abs(int x) {
    return x < 0 ? -x : x;
}

/// Compute the square of a number.
double square(double x) {
    return x * x;
}

/// A simple 2D point.
struct Point {
    double x;
    double y;
};

} // namespace math

/// Nested namespace using C++17 syntax.
namespace utils::string {

/// Trim whitespace from both ends.
std::string trim(const std::string& s) {
    auto start = s.find_first_not_of(" \t\n");
    auto end = s.find_last_not_of(" \t\n");
    if (start == std::string::npos) return "";
    return s.substr(start, end - start + 1);
}

/// Convert string to uppercase.
std::string to_upper(const std::string& s) {
    std::string result = s;
    std::transform(result.begin(), result.end(), result.begin(), ::toupper);
    return result;
}

} // namespace utils::string

/// Anonymous namespace for internal linkage.
namespace {
    int internal_counter = 0;

    void increment_counter() {
        ++internal_counter;
    }
}

/* ===== Struct with Methods ===== */

/// A simple counter with increment/decrement.
struct Counter {
    int value;

    Counter() : value(0) {}
    explicit Counter(int v) : value(v) {}

    void increment() { ++value; }
    void decrement() { --value; }
    int get() const { return value; }
};

/* ===== Abstract Base Class (Interface) ===== */

/// Abstract shape interface.
class Shape {
public:
    /// Get the area of the shape.
    virtual double area() const = 0;

    /// Get the perimeter of the shape.
    virtual double perimeter() const = 0;

    /// Get the name of the shape.
    virtual std::string name() const = 0;

    /// Virtual destructor.
    virtual ~Shape() = default;
};

/* ===== Derived Class: Circle ===== */

/// A circle shape.
class Circle : public Shape {
public:
    explicit Circle(double r) : radius_(r) {}

    double area() const override { return PI * radius_ * radius_; }
    double perimeter() const override { return 2.0 * PI * radius_; }
    std::string name() const override { return "Circle"; }

    double radius() const { return radius_; }

private:
    double radius_;
};

/* ===== Derived Class: Rectangle ===== */

/// A rectangle shape.
class Rectangle : public Shape {
public:
    Rectangle(double w, double h) : width_(w), height_(h) {}

    double area() const override { return width_ * height_; }
    double perimeter() const override { return 2.0 * (width_ + height_); }
    std::string name() const override { return "Rectangle"; }

    double width() const { return width_; }
    double height() const { return height_; }

protected:
    double width_;
    double height_;
};

/* ===== Final Class ===== */

/// A square shape (final, cannot be further derived).
class Square final : public Rectangle {
public:
    explicit Square(double side) : Rectangle(side, side) {}

    std::string name() const override { return "Square"; }
};

/* ===== Multiple Inheritance ===== */

/// Serializable interface.
class Serializable {
public:
    virtual std::string serialize() const = 0;
    virtual ~Serializable() = default;
};

/// Printable interface.
class Printable {
public:
    virtual void print(std::ostream& os) const = 0;
    virtual ~Printable() = default;
};

/// A class with multiple inheritance.
class Document : public Serializable, public Printable {
public:
    Document(std::string title, std::string content)
        : title_(std::move(title)), content_(std::move(content)) {}

    std::string serialize() const override {
        return title_ + ": " + content_;
    }

    void print(std::ostream& os) const override {
        os << "[Document] " << title_ << std::endl;
    }

    const std::string& title() const { return title_; }

private:
    std::string title_;
    std::string content_;
};

/* ===== Class with Operator Overloading ===== */

/// A 2D vector with operator overloading.
class Vec2 {
public:
    double x, y;

    Vec2() : x(0), y(0) {}
    Vec2(double x, double y) : x(x), y(y) {}

    Vec2 operator+(const Vec2& other) const {
        return Vec2(x + other.x, y + other.y);
    }

    Vec2 operator-(const Vec2& other) const {
        return Vec2(x - other.x, y - other.y);
    }

    Vec2 operator*(double scalar) const {
        return Vec2(x * scalar, y * scalar);
    }

    bool operator==(const Vec2& other) const {
        return x == other.x && y == other.y;
    }

    bool operator!=(const Vec2& other) const {
        return !(*this == other);
    }

    /// Subscript operator.
    double& operator[](int idx) {
        return idx == 0 ? x : y;
    }

    /// Stream output operator.
    friend std::ostream& operator<<(std::ostream& os, const Vec2& v);
};

std::ostream& operator<<(std::ostream& os, const Vec2& v) {
    os << "(" << v.x << ", " << v.y << ")";
    return os;
}

/* ===== RAII / Resource Management ===== */

/// RAII resource guard with deleted copy and defaulted move.
class ResourceGuard {
public:
    explicit ResourceGuard(int id) : id_(id), active_(true) {}
    ~ResourceGuard() { release(); }

    // Delete copy
    ResourceGuard(const ResourceGuard&) = delete;
    ResourceGuard& operator=(const ResourceGuard&) = delete;

    // Default move
    ResourceGuard(ResourceGuard&& other) = default;
    ResourceGuard& operator=(ResourceGuard&& other) = default;

    void release() {
        if (active_) {
            active_ = false;
        }
    }

    int id() const { return id_; }
    bool active() const { return active_; }

private:
    int id_;
    bool active_;
};

/* ===== Friend Class and Function ===== */

/// A class with private data accessible by friends.
class SecretHolder {
    friend class Inspector;
    friend void reveal_secret(const SecretHolder& holder);

public:
    SecretHolder(int secret) : secret_(secret) {}

private:
    int secret_;
};

/// Inspector can access SecretHolder's private members.
class Inspector {
public:
    static int inspect(const SecretHolder& holder) {
        return holder.secret_;
    }
};

/// Friend function that reveals the secret.
void reveal_secret(const SecretHolder& holder) {
    std::cout << "Secret: " << holder.secret_ << std::endl;
}

/* ===== Templates ===== */

/// A generic container template.
template<typename T>
class Container {
public:
    Container() = default;
    explicit Container(T val) : value_(std::move(val)) {}

    const T& get() const { return value_; }
    void set(T val) { value_ = std::move(val); }

    bool empty() const { return false; }

private:
    T value_;
};

/// Template specialization for void.
template<>
class Container<void> {
public:
    Container() = default;
    bool empty() const { return true; }
};

/// Generic add function template.
template<typename T>
T generic_add(T a, T b) {
    return a + b;
}

/// Variadic print function template.
template<typename... Args>
void print_all(Args&&... args) {
    (std::cout << ... << args) << std::endl;
}

/// A pair template.
template<typename T, typename U>
struct Pair {
    T first;
    U second;

    Pair(T f, U s) : first(std::move(f)), second(std::move(s)) {}
};

/* ===== Concepts (C++20) ===== */

/// Concept: type must support stream insertion.
template<typename T>
concept StreamInsertable = requires(std::ostream& os, T val) {
    { os << val } -> std::same_as<std::ostream&>;
};

/// Concept: type must support addition.
template<typename T>
concept Addable = requires(T a, T b) {
    { a + b } -> std::convertible_to<T>;
};

/// Constrained function using concept.
template<Addable T>
T constrained_add(T a, T b) {
    return a + b;
}

/* ===== Constexpr Functions ===== */

/// Compile-time factorial.
constexpr int factorial(int n) {
    return n <= 1 ? 1 : n * factorial(n - 1);
}

/// Compile-time square.
constexpr int compile_time_square(int x) {
    return x * x;
}

/* ===== Consteval Function (C++20) ===== */

/// Must be evaluated at compile time.
consteval int compile_only_double(int x) {
    return x * 2;
}

/* ===== Noexcept Functions ===== */

/// A function that does not throw.
int safe_divide(int a, int b) noexcept {
    return b != 0 ? a / b : 0;
}

/* ===== Trailing Return Type ===== */

/// Function with trailing return type.
auto trailing_return(int a, int b) -> int {
    return a + b;
}

/* ===== Inline Function ===== */

/// Inline utility function.
inline int fast_max(int a, int b) {
    return a > b ? a : b;
}

/* ===== Static Function ===== */

/// File-scoped helper.
static int internal_helper(int x) {
    return x * 2;
}

/* ===== Extern Prototype ===== */
extern int external_function(int arg);

/* ===== Lambda Expressions ===== */

/// A lambda stored as a variable.
auto doubler = [](int x) { return x * 2; };

/// A lambda factory.
auto make_adder(int base) -> std::function<int(int)> {
    return [base](int x) { return base + x; };
}

/* ===== Nested Class ===== */

/// Outer class with a nested inner class.
class Outer {
public:
    /// Inner class.
    class Inner {
    public:
        int value;
        Inner(int v) : value(v) {}
    };

    Inner create_inner(int v) const {
        return Inner(v);
    }

private:
    int data_;
};

/* ===== Struct with Template ===== */

/// A templated node for a linked list.
template<typename T>
struct ListNode {
    T data;
    ListNode* next;

    explicit ListNode(T val) : data(std::move(val)), next(nullptr) {}
};

/* ===== User-Defined Literal ===== */

/// User-defined literal for kilometers.
long double operator""_km(long double val) {
    return val * 1000.0L;
}

/* ===== Global Variables ===== */

/// Global configuration string.
const std::string APP_NAME = "TestApp";

/// Mutable global counter.
int g_counter = 0;

/// Static global.
static int s_instance_count = 0;

/// Extern global (declaration only).
extern int shared_value;

/* ===== Typedef (C-style) ===== */
typedef void (*OldCallback)(int, int);

/* ===== Free Function with Multiple Params ===== */

/// Process data with multiple parameters.
int process_data(const std::string& input, int flags, double threshold) {
    if (input.empty()) return -1;
    return flags > 0 ? static_cast<int>(threshold) : 0;
}

/* ===== Conversion Operator ===== */

/// A wrapper with implicit conversion.
class IntWrapper {
public:
    explicit IntWrapper(int v) : val_(v) {}

    /// Convert to int.
    operator int() const { return val_; }

    /// Convert to bool.
    explicit operator bool() const { return val_ != 0; }

private:
    int val_;
};

/* ===== Virtual Inheritance ===== */

/// Base class for diamond inheritance.
class VBase {
public:
    int base_val;
    VBase() : base_val(0) {}
    virtual ~VBase() = default;
};

/// Left branch of diamond.
class VLeft : virtual public VBase {
public:
    void set_left(int v) { base_val = v; }
};

/// Right branch of diamond.
class VRight : virtual public VBase {
public:
    void set_right(int v) { base_val = v + 1; }
};

/// Diamond tip.
class Diamond : public VLeft, public VRight {
public:
    int combined() const { return base_val; }
};

/* ===== Explicit Specifier ===== */

/// A class with an explicit constructor.
class ExplicitOnly {
public:
    explicit ExplicitOnly(int v) : val_(v) {}
    int value() const { return val_; }

private:
    int val_;
};

/* ===== Qualified Identifier (out-of-class method definition) ===== */

/// A class with out-of-class method definitions.
class OutOfClass {
public:
    void method_a();
    int method_b(int x);

private:
    int data_;
};

void OutOfClass::method_a() {
    data_ = 42;
}

int OutOfClass::method_b(int x) {
    return data_ + x;
}

/* ===== C++11 Attributes ===== */

/// A function with [[nodiscard]] attribute.
[[nodiscard]] int must_use_result(int x) {
    return x * 2;
}

/// A function with [[deprecated]] attribute.
[[deprecated("use new_api instead")]] void old_api() {}

/// A variable with an attribute.
[[maybe_unused]] static int unused_var = 99;

/* ===== Access Specifier Tracking ===== */

/// A class demonstrating access specifiers.
class AccessDemo {
public:
    int pub_field;
    void pub_method() {}

protected:
    int prot_field;
    void prot_method() {}

private:
    int priv_field;
    void priv_method() {}
};

/* ===== Template Alias ===== */

/// A template alias.
template<typename T>
using SharedPtr = std::shared_ptr<T>;

/// Another template alias with two params.
template<typename K, typename V>
using Map = std::vector<std::pair<K, V>>;

/* ===== Namespace Alias ===== */

namespace very_long_namespace_name {
    int helper() { return 1; }
}

/// Namespace alias for convenience.
namespace vln = very_long_namespace_name;

/* ===== Friend Declarations ===== */

/// A class with friend class and friend function declarations.
class FriendDemo {
    friend class FriendClass;
    friend void friend_func(FriendDemo& fd);
public:
    FriendDemo(int v) : value_(v) {}
private:
    int value_;
};

/* ===== Nested Types in Class ===== */

/// A class with nested types: enum, struct, alias, static_assert.
class NestingDemo {
public:
    /// Nested enum inside class.
    enum class InnerStatus { OK, Error, Pending };

    /// Nested struct inside class.
    struct InnerConfig {
        int timeout;
        bool verbose;
    };

    /// Nested using alias inside class.
    using InnerCallback = std::function<void()>;

    void do_work() {}

private:
    InnerConfig config_;
};

/* ===== Method Qualifiers (override, final, = delete, = default, const) ===== */

/// Base class for method qualifier testing.
class MethodBase {
public:
    virtual void normal_virtual() {}
    virtual void overridden() {}
    virtual void final_method() {}
    virtual ~MethodBase() = default;
};

/// Derived class demonstrating override/final/deleted/defaulted methods.
class MethodDerived : public MethodBase {
public:
    void overridden() override {}
    void final_method() final {}

    // Deleted and defaulted
    MethodDerived() = default;
    MethodDerived(const MethodDerived&) = delete;
    MethodDerived& operator=(const MethodDerived&) = delete;
    MethodDerived(MethodDerived&&) = default;

    // Const method
    int get_value() const { return 42; }
};

/* ===== Inline Namespace ===== */

/// An inline namespace for versioning.
inline namespace v2 {
    int versioned_func() { return 2; }
}

/* ===== Explicit Template Instantiation ===== */

/// Explicit template instantiation.
template class Container<int>;

/* ===== Requires Clause ===== */

/// A function with a requires clause.
template<typename T>
    requires std::is_integral_v<T>
T checked_add(T a, T b) {
    return a + b;
}

/* ===== Structured Bindings ===== */

/// Function using structured bindings.
void use_structured_bindings() {
    struct Point2 { int x; int y; };
    Point2 pt{10, 20};
    auto [sx, sy] = pt;
    (void)sx; (void)sy;
}

/* ===== Union Inside Namespace ===== */

namespace geo {

/// A tagged union for shapes.
union ShapeData {
    double radius;
    double side_length;
    struct { double w; double h; } rect;
};

int area_calc(int x) { return x * x; }

} // namespace geo

/* ===== C++20 Modules (simulated with comments — tree-sitter may not parse) ===== */
// NOTE: actual module syntax may not parse with all tree-sitter-cpp versions

/* ===== Coroutine-like markers ===== */

// Coroutines require full library support; we test the attribute detection
// in the enrichment pass with a simple marker pattern instead.

/* ===== Decltype Return Type ===== */

/// Function using decltype return type.
auto decltype_example(int a, double b) -> decltype(a + b) {
    return a + b;
}

/* ===== Using Directive ===== */

/// Using directive (using namespace).
using namespace std;

/* ===== Deduction Guide ===== */

template<typename T>
struct Wrapper {
    T value;
    Wrapper(T v) : value(v) {}
};

// Deduction guide for Wrapper
Wrapper(const char*) -> Wrapper<std::string>;

/* ===== Out-of-class template method ===== */

/// A class with a template method defined out-of-class.
class TemplateMethodHost {
public:
    template<typename T>
    T convert(int x);
};

/* ===== Attributed Declarator ===== */

/// A nodiscard class.
class [[nodiscard]] MustUseClass {
public:
    int val;
    MustUseClass(int v) : val(v) {}
};
