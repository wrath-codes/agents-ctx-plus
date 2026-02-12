/**
 * @file sample.c
 * @brief Comprehensive C fixture for testing the rich extractor.
 *
 * This file contains 40+ distinct top-level items covering all major
 * C language constructs: functions, structs, unions, enums, typedefs,
 * global variables, constants, preprocessor directives, function pointers,
 * bit fields, array declarations, forward declarations, doc comments,
 * header guards, and static assertions.
 */

/* ===== Header Guards ===== */
#ifndef SAMPLE_H
#define SAMPLE_H

/* ===== Includes (system) ===== */
#include <stdio.h>
#include <stdlib.h>
#include <stdarg.h>
#include <string.h>
#include <stdint.h>

/* ===== Includes (local) ===== */
#include "mylib.h"
#include "utils.h"

/* ===== Pragma ===== */
#pragma once
#pragma pack(push, 1)

/* ===== Object-like Macros ===== */
#define MAX_BUFFER 1024
#define VERSION_MAJOR 2
#define VERSION_MINOR 7
#define PI 3.14159265358979

/* ===== Function-like Macros ===== */
#define SQUARE(x) ((x) * (x))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define DEBUG_LOG(fmt, ...) fprintf(stderr, "[DEBUG] " fmt "\n", __VA_ARGS__)

/* ===== Conditional Compilation ===== */
#ifdef DEBUG
#define TRACE(msg) printf("TRACE: %s\n", msg)
#else
#define TRACE(msg) ((void)0)
#endif

#ifndef NDEBUG
#define ASSERT(cond) do { if (!(cond)) abort(); } while(0)
#endif

#if defined(__GNUC__) && __GNUC__ >= 4
#define EXPORT __attribute__((visibility("default")))
#else
#define EXPORT
#endif

/* ===== #if Expression ===== */
#if __STDC_VERSION__ >= 201112L
int c11_available = 1;
#elif __STDC_VERSION__ >= 199901L
int c99_available = 1;
#else
int c89_only = 1;
#endif

/* ===== Forward Declarations ===== */

/** Forward declaration of struct Node (used in linked list) */
struct Node;

/** Forward declaration of enum Status */
enum Status;

/** Forward declaration of struct OpaqueHandle */
struct OpaqueHandle;

/* ===== Static Assertions ===== */

/** Verify int is 4 bytes on this platform */
_Static_assert(sizeof(int) == 4, "int must be 4 bytes");

/** Verify pointer size is 8 bytes (64-bit) */
_Static_assert(sizeof(void*) == 8, "expected 64-bit pointers");

/* ===== Enums ===== */

/**
 * @brief Color constants for rendering.
 *
 * Represents primary RGB color channels.
 */
enum Color {
    COLOR_RED   = 0xFF0000,
    COLOR_GREEN = 0x00FF00,
    COLOR_BLUE  = 0x0000FF,
    COLOR_WHITE = 0xFFFFFF,
    COLOR_BLACK = 0x000000
};

/**
 * Status codes returned by processing functions.
 */
typedef enum {
    STATUS_OK       = 0,
    STATUS_ERROR    = -1,
    STATUS_PENDING  = 1,
    STATUS_TIMEOUT  = 2,
    STATUS_CANCELED = 3
} StatusCode;

// Log level enum using C++ style comment
enum LogLevel {
    LOG_TRACE,
    LOG_DEBUG,
    LOG_INFO,
    LOG_WARN,
    LOG_ERROR,
    LOG_FATAL
};

/* ===== Structs ===== */

/** A 2D point with integer coordinates. */
struct Point {
    int x;  /**< X coordinate */
    int y;  /**< Y coordinate */
};

/**
 * @brief A rectangle defined by origin and size.
 */
typedef struct {
    struct Point origin;
    unsigned int width;
    unsigned int height;
} Rectangle;

/**
 * Linked list node holding an integer value.
 */
struct Node {
    int value;
    struct Node *next;
    struct Node *prev;
};

/**
 * @brief Hardware register with bit fields.
 *
 * Demonstrates bit-field extraction for register-level access.
 */
struct HardwareRegister {
    unsigned int enabled    : 1;   /**< Enable bit */
    unsigned int mode       : 3;   /**< Operating mode (0-7) */
    unsigned int priority   : 4;   /**< Interrupt priority (0-15) */
    unsigned int reserved   : 8;   /**< Reserved bits */
    unsigned int error_code : 16;  /**< Error code field */
};

/**
 * Configuration structure with nested types and arrays.
 */
struct Config {
    char name[64];
    char hostname[256];
    int port;
    int max_connections;
    double timeout_secs;
    enum LogLevel log_level;
    unsigned char flags;
};

/** Struct with an anonymous union member. */
struct TaggedValue {
    int tag;
    union {
        int as_int;
        float as_float;
        char as_str[16];
    };
};

/* ===== Unions ===== */

/**
 * @brief A tagged value that can hold different primitive types.
 *
 * Use alongside a type tag to interpret which field is active.
 */
union Value {
    int    as_int;
    float  as_float;
    double as_double;
    char   as_string[32];
    void  *as_pointer;
};

/** Network address union for IPv4 / IPv6 */
union NetworkAddress {
    uint32_t ipv4;
    uint8_t  ipv6[16];
    char     hostname[128];
};

/* ===== Typedefs ===== */

/** Simple type alias for byte */
typedef unsigned char Byte;

/** Type alias for a size value */
typedef unsigned long Size;

/** Typedef for struct Point */
typedef struct Point Point2D;

/** Function pointer typedef: a comparator */
typedef int (*Comparator)(const void *, const void *);

/** Function pointer typedef: an event callback */
typedef void (*EventCallback)(int event_type, void *user_data);

/** Function pointer typedef: an allocator */
typedef void *(*Allocator)(Size size);

/* ===== Global Variables ===== */

/** Global counter, initialized to zero. */
int global_counter = 0;

/** Static internal state, not visible outside this TU. */
static int internal_state = -1;

/** Extern-linked shared value from another translation unit. */
extern int shared_value;

/** Compile-time constant for maximum items. */
const int MAX_ITEMS = 256;

/** Another constant: default timeout in milliseconds. */
const double DEFAULT_TIMEOUT_MS = 5000.0;

/** Static constant string tag. */
static const char *BUILD_TAG = "v2.7.0-fixture";

/** Hardware sensor reading, may change unexpectedly. */
volatile int sensor_reading;

/** Volatile constant hardware address. */
volatile const int HW_STATUS_REG = 0xDEAD;

/** Register hint for frequently accessed counter. */
register int fast_counter;

/** Multiple variables in one declaration. */
int multi_a = 10, multi_b = 20, multi_c = 30;

/** Comma-separated identifiers without init. */
int coord_x, coord_y, coord_z;

/** GCC attribute on a variable. */
__attribute__((unused)) static int attr_var = 0;

/** GCC attribute on a function. */
__attribute__((noreturn)) void panic_handler(const char *msg);

/** Noreturn function that aborts. */
_Noreturn void abort_with_message(const char *msg);

/** Atomic counter for thread safety. */
_Atomic int atomic_counter;

/** Environment pointer array. */
char **environment;

/* ===== Array Declarations ===== */

/** Lookup table of 256 byte values. */
int lookup_table[256];

/** Pre-initialized small array. */
static int prime_numbers[10] = {2, 3, 5, 7, 11, 13, 17, 19, 23, 29};

/** Multi-dimensional matrix. */
double transform_matrix[4][4];

/* ===== Function Pointer Variables ===== */

/**
 * Global callback function pointer.
 *
 * Assign a handler before calling dispatch().
 */
void (*on_event_callback)(int, int) = NULL;

/** Cleanup handler, called at program exit. */
static void (*cleanup_handler)(void) = NULL;

/* ===== Function Declarations (Prototypes) ===== */

/** Compute the sum of two integers. */
int add(int a, int b);

/** Subtract b from a. */
int subtract(int a, int b);

/** Process a data buffer of given length. Returns status code. */
StatusCode process_data(const char *buffer, Size length);

/** Initialize the subsystem with a config. */
int initialize_subsystem(const struct Config *cfg);

/** Shutdown and release resources. */
void shutdown_subsystem(void);

/* ===== Function Definitions ===== */

/**
 * @brief Add two integers and return the result.
 *
 * @param a First operand.
 * @param b Second operand.
 * @return Sum of a and b.
 */
int add(int a, int b) {
    return a + b;
}

/**
 * @brief Subtract b from a.
 *
 * @param a Minuend.
 * @param b Subtrahend.
 * @return Difference (a - b).
 */
int subtract(int a, int b) {
    return a - b;
}

/**
 * @brief Internal helper — clamps a value to [lo, hi].
 *
 * This is a static inline helper that should not be exported.
 */
static inline int clamp_value(int value, int lo, int hi) {
    if (value < lo) return lo;
    if (value > hi) return hi;
    return value;
}

/**
 * @brief Multiply two integers (extern linkage, used in other TUs).
 */
extern int multiply(int a, int b) {
    return a * b;
}

/**
 * Process a data buffer, validating and transforming its contents.
 *
 * @param buffer  Pointer to input data.
 * @param length  Number of bytes in the buffer.
 * @return STATUS_OK on success, STATUS_ERROR on failure.
 */
StatusCode process_data(const char *buffer, Size length) {
    if (!buffer || length == 0) {
        return STATUS_ERROR;
    }
    global_counter += (int)length;
    TRACE("processing data");
    return STATUS_OK;
}

/**
 * @brief Log a formatted message with variable arguments.
 *
 * Demonstrates variadic function parameters.
 *
 * @param level   Log level for this message.
 * @param fmt     printf-style format string.
 * @param ...     Variable arguments matching fmt.
 */
void variadic_log(enum LogLevel level, const char *fmt, ...) {
    const char *level_names[] = {
        "TRACE", "DEBUG", "INFO", "WARN", "ERROR", "FATAL"
    };
    va_list args;
    va_start(args, fmt);
    fprintf(stderr, "[%s] ", level_names[level]);
    vfprintf(stderr, fmt, args);
    fprintf(stderr, "\n");
    va_end(args);
}

/**
 * @brief Create a new Point.
 *
 * Factory function returning a struct by value.
 */
struct Point make_point(int x, int y) {
    struct Point p;
    p.x = x;
    p.y = y;
    return p;
}

/**
 * @brief Compute the area of a rectangle.
 *
 * @param rect  Pointer to a Rectangle.
 * @return Area in square units.
 */
unsigned int rectangle_area(const Rectangle *rect) {
    if (!rect) return 0;
    return rect->width * rect->height;
}

/**
 * @brief Compare two integers (for use with qsort).
 *
 * Matches the Comparator typedef signature.
 */
int int_comparator(const void *a, const void *b) {
    int ia = *(const int *)a;
    int ib = *(const int *)b;
    return (ia > ib) - (ia < ib);
}

/**
 * @brief Initialize the subsystem from a Config struct.
 *
 * @param cfg  Configuration parameters.
 * @return 0 on success, -1 on failure.
 */
int initialize_subsystem(const struct Config *cfg) {
    if (!cfg) return -1;
    internal_state = 1;
    variadic_log(LOG_INFO, "Subsystem initialized: %s on port %d",
                 cfg->name, cfg->port);
    return 0;
}

/**
 * @brief Shutdown the subsystem and release resources.
 */
void shutdown_subsystem(void) {
    if (cleanup_handler) {
        cleanup_handler();
    }
    internal_state = 0;
    variadic_log(LOG_INFO, "Subsystem shut down (counter=%d)", global_counter);
}

/* ===== Pragma Pop ===== */
#pragma pack(pop)

/* ===== End Header Guard ===== */
#endif /* SAMPLE_H */
