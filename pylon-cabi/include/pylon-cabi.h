/* Plain-C interface for the pylon-cxx shim shared library.
 *
 * All functions that can fail return NULL on success, or a malloc-allocated
 * error string on failure.  Callers must free error strings with
 * pylon_cxx_free_str().  All "out_*" pointer-to-object parameters receive
 * heap-allocated objects that the caller must free with the matching
 * *_destroy() function.  "const void*" node-map pointers are borrowed —
 * they are valid only as long as the owning camera / grab-result is alive,
 * and must NOT be passed to any destroy function.
 */
#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define PYLON_CABI_VERSION 1

typedef struct pylon_cabi_api pylon_cabi_api;
typedef struct pylon_cabi_api pylon_shim_api;

/* --- Memory management -------------------------------------------------- */
void pylon_cxx_free_str(char *s);
/* Free a malloc-allocated array (e.g. the void** from enumerate_devices). */
void pylon_cxx_free_ptr(void *p);

/* --- Pylon lifecycle ----------------------------------------------------- */
void pylon_initialize(void);
void pylon_terminate(int shutdown_logging);
void pylon_get_version(uint32_t *major, uint32_t *minor,
                       uint32_t *subminor, uint32_t *build);

/* --- TlFactory ----------------------------------------------------------- */
/* Creates a CInstantCamera on the heap.  Caller frees with
 * instant_camera_destroy(). */
const char *tl_factory_create_first_device(void **out_camera);
const char *tl_factory_create_device(const void *device_info,
                                     void **out_camera);

/* Enumerates devices.  *out_infos receives a malloc-allocated array of
 * *out_len heap-allocated CDeviceInfo pointers.  Caller must free each
 * element with device_info_destroy(), then free the array with free(). */
const char *tl_factory_enumerate_devices(void ***out_infos, size_t *out_len);

/* --- CInstantCamera ------------------------------------------------------ */
void instant_camera_destroy(void *camera);
/* Returns a new heap-allocated CDeviceInfo (freed by caller). */
const char *instant_camera_get_device_info(const void *camera,
                                           void **out_device_info);
const char *instant_camera_open(void *camera);
const char *instant_camera_is_open(const void *camera, int *out);
const char *instant_camera_close(void *camera);
const char *instant_camera_start_grabbing(void *camera);
const char *instant_camera_stop_grabbing(void *camera);
int         instant_camera_is_grabbing(const void *camera);
const char *instant_camera_start_grabbing_with_strategy(void *camera,
                                                         int grab_strategy);
const char *instant_camera_start_grabbing_with_count(void *camera,
                                                      uint32_t count);
const char *instant_camera_start_grabbing_with_count_and_strategy(
    void *camera, uint32_t count, int grab_strategy);
/* out_grabbed: 1 if result available, 0 on timeout (Return mode) */
const char *instant_camera_retrieve_result(void *camera,
                                           uint32_t timeout_ms,
                                           void *grab_result,
                                           int timeout_handling,
                                           int *out_grabbed);

/* Node-map accessors — borrowed pointers, do NOT destroy. */
const char *instant_camera_get_node_map(const void *camera,
                                        const void **out_node_map);
const char *instant_camera_get_tl_node_map(const void *camera,
                                           const void **out_node_map);
const char *instant_camera_get_stream_grabber_node_map(
    const void *camera, const void **out_node_map);
const char *instant_camera_get_event_grabber_node_map(
    const void *camera, const void **out_node_map);
const char *instant_camera_get_instant_camera_node_map(
    const void *camera, const void **out_node_map);

#if !defined(_WIN32)
/* Unix stream support: returns the grab-result wait-object file descriptor. */
const char *instant_camera_wait_object_fd(const void *camera, int *out_fd);
#endif

#if defined(_WIN32)
/* Windows stream support: returns a heap-allocated WaitObject. */
const char *instant_camera_wait_object(const void *camera,
                                       void **out_wait_object);
#endif

/* --- NodeMap operations -------------------------------------------------- */
const char *node_map_load(const void *node_map, const char *filename,
                          size_t filename_len, int validate);
const char *node_map_save(const void *node_map, const char *filename,
                          size_t filename_len);
const char *node_map_load_from_string(const void *node_map,
                                      const char *features,
                                      size_t features_len, int validate);
/* *out_str is a malloc-allocated string; free with pylon_cxx_free_str(). */
const char *node_map_save_to_string(const void *node_map, char **out_str);

/* Parameter accessors — callers free with the matching *_destroy(). */
const char *node_map_get_boolean_parameter(const void *node_map,
                                           const char *name, size_t name_len,
                                           void **out);
const char *node_map_get_integer_parameter(const void *node_map,
                                           const char *name, size_t name_len,
                                           void **out);
const char *node_map_get_float_parameter(const void *node_map,
                                         const char *name, size_t name_len,
                                         void **out);
const char *node_map_get_enum_parameter(const void *node_map,
                                        const char *name, size_t name_len,
                                        void **out);
const char *node_map_get_command_parameter(const void *node_map,
                                           const char *name, size_t name_len,
                                           void **out);

/* --- CBooleanParameter --------------------------------------------------- */
void        boolean_parameter_destroy(void *p);
const char *boolean_node_get_value(const void *p, int *out);
const char *boolean_node_set_value(void *p, int value);

/* --- CIntegerParameter --------------------------------------------------- */
void        integer_parameter_destroy(void *p);
/* *out: malloc-allocated; free with pylon_cxx_free_str(). */
const char *integer_node_get_unit(const void *p, char **out);
const char *integer_node_get_value(const void *p, int64_t *out);
const char *integer_node_get_min(const void *p, int64_t *out);
const char *integer_node_get_max(const void *p, int64_t *out);
const char *integer_node_set_value(void *p, int64_t value);

/* --- CFloatParameter ----------------------------------------------------- */
void        float_parameter_destroy(void *p);
const char *float_node_get_unit(const void *p, char **out);
const char *float_node_get_value(const void *p, double *out);
const char *float_node_get_min(const void *p, double *out);
const char *float_node_get_max(const void *p, double *out);
const char *float_node_set_value(void *p, double value);

/* --- CEnumParameter ------------------------------------------------------ */
void        enum_parameter_destroy(void *p);
const char *enum_node_get_value(const void *p, char **out);
/* *out_values: malloc-allocated array of *out_count malloc-allocated strings.
 * Free with enum_node_free_settable_values(). */
const char *enum_node_settable_values(const void *p, char ***out_values,
                                      size_t *out_count);
void        enum_node_free_settable_values(char **values, size_t count);
const char *enum_node_set_value(void *p, const char *value,
                                size_t value_len);

/* --- CCommandParameter --------------------------------------------------- */
void        command_parameter_destroy(void *p);
const char *command_node_execute(void *p, int verify);

/* --- CGrabResultPtr ------------------------------------------------------ */
const char *new_grab_result_ptr(void **out);
void        grab_result_ptr_destroy(void *p);
const char *grab_result_grab_succeeded(const void *p, int *out);
const char *grab_result_error_description(const void *p, char **out);
const char *grab_result_error_code(const void *p, uint32_t *out);
const char *grab_result_width(const void *p, uint32_t *out);
const char *grab_result_height(const void *p, uint32_t *out);
const char *grab_result_offset_x(const void *p, uint32_t *out);
const char *grab_result_offset_y(const void *p, uint32_t *out);
const char *grab_result_padding_x(const void *p, uint32_t *out);
const char *grab_result_padding_y(const void *p, uint32_t *out);
/* *out_buf is a borrowed pointer into the grab result — valid only while p
 * is alive.  Do NOT free it. */
const char *grab_result_buffer(const void *p, const uint8_t **out_buf,
                               size_t *out_len);
const char *grab_result_payload_size(const void *p, uint32_t *out);
const char *grab_result_buffer_size(const void *p, uint32_t *out);
const char *grab_result_block_id(const void *p, uint64_t *out);
const char *grab_result_time_stamp(const void *p, uint64_t *out);
const char *grab_result_stride(const void *p, size_t *out);
const char *grab_result_image_size(const void *p, uint32_t *out);
/* Borrowed node-map pointer — valid while p is alive. */
const char *grab_result_get_chunk_data_node_map(const void *p,
                                                const void **out_node_map);

/* --- CDeviceInfo --------------------------------------------------------- */
void        device_info_destroy(void *p);
/* Returns a newly heap-allocated copy (freed with device_info_destroy()). */
const char *device_info_clone(const void *p, void **out);
/* *out_names: malloc array of *out_count malloc strings; free with
 * device_info_free_property_names(). */
const char *device_info_get_property_names(const void *p,
                                           char ***out_names,
                                           size_t *out_count);
void        device_info_free_property_names(char **names, size_t count);
/* *out: malloc string; free with pylon_cxx_free_str(). */
const char *device_info_get_property_value(const void *p,
                                           const char *name, size_t name_len,
                                           char **out);
const char *device_info_get_model_name(const void *p, char **out);

#if defined(_WIN32)
/* --- WaitObject (Windows stream) ---------------------------------------- */
void        wait_object_destroy(void *p);
const char *wait_object_wait(const void *p, uint64_t timeout_ms, int *out);
#endif

struct pylon_cabi_api {
    uint32_t abi_version;
    uint32_t struct_size;

    void (*pylon_cxx_free_str)(char *s);
    void (*pylon_cxx_free_ptr)(void *p);

    void (*pylon_initialize)(void);
    void (*pylon_terminate)(int shutdown_logging);
    void (*pylon_get_version)(uint32_t *major, uint32_t *minor,
                              uint32_t *subminor, uint32_t *build);

    const char *(*tl_factory_create_first_device)(void **out_camera);
    const char *(*tl_factory_create_device)(const void *device_info,
                                            void **out_camera);
    const char *(*tl_factory_enumerate_devices)(void ***out_infos, size_t *out_len);

    void (*instant_camera_destroy)(void *camera);
    const char *(*instant_camera_get_device_info)(const void *camera,
                                                   void **out_device_info);
    const char *(*instant_camera_open)(void *camera);
    const char *(*instant_camera_is_open)(const void *camera, int *out);
    const char *(*instant_camera_close)(void *camera);
    const char *(*instant_camera_start_grabbing)(void *camera);
    const char *(*instant_camera_stop_grabbing)(void *camera);
    int (*instant_camera_is_grabbing)(const void *camera);
    const char *(*instant_camera_start_grabbing_with_strategy)(void *camera,
                                                                int grab_strategy);
    const char *(*instant_camera_start_grabbing_with_count)(void *camera,
                                                             uint32_t count);
    const char *(*instant_camera_start_grabbing_with_count_and_strategy)(
        void *camera, uint32_t count, int grab_strategy);
    const char *(*instant_camera_retrieve_result)(void *camera,
                                                   uint32_t timeout_ms,
                                                   void *grab_result,
                                                   int timeout_handling,
                                                   int *out_grabbed);
    const char *(*instant_camera_get_node_map)(const void *camera,
                                                const void **out_node_map);
    const char *(*instant_camera_get_tl_node_map)(const void *camera,
                                                   const void **out_node_map);
    const char *(*instant_camera_get_stream_grabber_node_map)(
        const void *camera, const void **out_node_map);
    const char *(*instant_camera_get_event_grabber_node_map)(
        const void *camera, const void **out_node_map);
    const char *(*instant_camera_get_instant_camera_node_map)(
        const void *camera, const void **out_node_map);

#if !defined(_WIN32)
    const char *(*instant_camera_wait_object_fd)(const void *camera, int *out_fd);
#endif

#if defined(_WIN32)
    const char *(*instant_camera_wait_object)(const void *camera,
                                              void **out_wait_object);
#endif

    const char *(*node_map_load)(const void *node_map, const char *filename,
                                 size_t filename_len, int validate);
    const char *(*node_map_save)(const void *node_map, const char *filename,
                                 size_t filename_len);
    const char *(*node_map_load_from_string)(const void *node_map,
                                             const char *features,
                                             size_t features_len, int validate);
    const char *(*node_map_save_to_string)(const void *node_map, char **out_str);
    const char *(*node_map_get_boolean_parameter)(const void *node_map,
                                                  const char *name,
                                                  size_t name_len,
                                                  void **out);
    const char *(*node_map_get_integer_parameter)(const void *node_map,
                                                  const char *name,
                                                  size_t name_len,
                                                  void **out);
    const char *(*node_map_get_float_parameter)(const void *node_map,
                                                const char *name,
                                                size_t name_len,
                                                void **out);
    const char *(*node_map_get_enum_parameter)(const void *node_map,
                                               const char *name,
                                               size_t name_len,
                                               void **out);
    const char *(*node_map_get_command_parameter)(const void *node_map,
                                                  const char *name,
                                                  size_t name_len,
                                                  void **out);

    void (*boolean_parameter_destroy)(void *p);
    const char *(*boolean_node_get_value)(const void *p, int *out);
    const char *(*boolean_node_set_value)(void *p, int value);

    void (*integer_parameter_destroy)(void *p);
    const char *(*integer_node_get_unit)(const void *p, char **out);
    const char *(*integer_node_get_value)(const void *p, int64_t *out);
    const char *(*integer_node_get_min)(const void *p, int64_t *out);
    const char *(*integer_node_get_max)(const void *p, int64_t *out);
    const char *(*integer_node_set_value)(void *p, int64_t value);

    void (*float_parameter_destroy)(void *p);
    const char *(*float_node_get_unit)(const void *p, char **out);
    const char *(*float_node_get_value)(const void *p, double *out);
    const char *(*float_node_get_min)(const void *p, double *out);
    const char *(*float_node_get_max)(const void *p, double *out);
    const char *(*float_node_set_value)(void *p, double value);

    void (*enum_parameter_destroy)(void *p);
    const char *(*enum_node_get_value)(const void *p, char **out);
    const char *(*enum_node_settable_values)(const void *p, char ***out_values,
                                             size_t *out_count);
    void (*enum_node_free_settable_values)(char **values, size_t count);
    const char *(*enum_node_set_value)(void *p, const char *value,
                                       size_t value_len);

    void (*command_parameter_destroy)(void *p);
    const char *(*command_node_execute)(void *p, int verify);

    const char *(*new_grab_result_ptr)(void **out);
    void (*grab_result_ptr_destroy)(void *p);
    const char *(*grab_result_grab_succeeded)(const void *p, int *out);
    const char *(*grab_result_error_description)(const void *p, char **out);
    const char *(*grab_result_error_code)(const void *p, uint32_t *out);
    const char *(*grab_result_width)(const void *p, uint32_t *out);
    const char *(*grab_result_height)(const void *p, uint32_t *out);
    const char *(*grab_result_offset_x)(const void *p, uint32_t *out);
    const char *(*grab_result_offset_y)(const void *p, uint32_t *out);
    const char *(*grab_result_padding_x)(const void *p, uint32_t *out);
    const char *(*grab_result_padding_y)(const void *p, uint32_t *out);
    const char *(*grab_result_buffer)(const void *p, const uint8_t **out_buf,
                                      size_t *out_len);
    const char *(*grab_result_payload_size)(const void *p, uint32_t *out);
    const char *(*grab_result_buffer_size)(const void *p, uint32_t *out);
    const char *(*grab_result_block_id)(const void *p, uint64_t *out);
    const char *(*grab_result_time_stamp)(const void *p, uint64_t *out);
    const char *(*grab_result_stride)(const void *p, size_t *out);
    const char *(*grab_result_image_size)(const void *p, uint32_t *out);
    const char *(*grab_result_get_chunk_data_node_map)(const void *p,
                                                       const void **out_node_map);

    void (*device_info_destroy)(void *p);
    const char *(*device_info_clone)(const void *p, void **out);
    const char *(*device_info_get_property_names)(const void *p,
                                                  char ***out_names,
                                                  size_t *out_count);
    void (*device_info_free_property_names)(char **names, size_t count);
    const char *(*device_info_get_property_value)(const void *p,
                                                  const char *name,
                                                  size_t name_len,
                                                  char **out);
    const char *(*device_info_get_model_name)(const void *p, char **out);

#if defined(_WIN32)
    void (*wait_object_destroy)(void *p);
    const char *(*wait_object_wait)(const void *p, uint64_t timeout_ms, int *out);
#endif
};

const pylon_cabi_api *pylon_cabi_get_api(void);
const pylon_shim_api *pylon_shim_get_api(void);

#ifdef __cplusplus
} /* extern "C" */
#endif
