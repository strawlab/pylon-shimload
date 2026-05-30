/* Plain-C implementation of the pylon-cxx shim interface.
 *
 * Compiled once by the maintainer (or locally by anyone with the pylon SDK)
 * into a shared library.  Downstream users load it at runtime via libloading.
 *
 * Error convention: all fallible functions return NULL on success, or a
 * malloc-allocated NUL-terminated error string on failure.
 */
#if !defined(_WIN32)
#include <unistd.h>
#include <fcntl.h>
#endif

#include <stdlib.h>
#include <string.h>
#include <memory>
#include <string>
#include <vector>
#include <exception>
#include <sstream>

#include "pylon/PylonIncludes.h"
#include "libpylon-cabi.h"

/* -------------------------------------------------------------------------
 * Internal helpers
 * ---------------------------------------------------------------------- */

static char *make_error(const char *msg)
{
    return strdup(msg);
}

#define SHIM_TRY try {
#define SHIM_CATCH \
    } catch (const Pylon::GenericException &e) { \
        return make_error(e.what()); \
    } catch (const std::exception &e) { \
        return make_error(e.what()); \
    } catch (...) { \
        return make_error("unknown C++ exception"); \
    }

// The pylon API is not const-correct: GetNodeMap(), GetValue(), CGrabResultPtr::operator*(),
// etc. are non-const member functions. The C interface still advertises `const void*` for
// caller-side read-only intent, but internally these helpers strip const so the underlying
// pylon methods are callable.
static Pylon::CInstantCamera *cam_of(void *p)
{
    return static_cast<Pylon::CInstantCamera *>(p);
}
static Pylon::CInstantCamera *cam_of(const void *p)
{
    return static_cast<Pylon::CInstantCamera *>(const_cast<void *>(p));
}
static Pylon::CGrabResultPtr *grab_of(void *p)
{
    return static_cast<Pylon::CGrabResultPtr *>(p);
}
static Pylon::CGrabResultPtr *grab_of(const void *p)
{
    return static_cast<Pylon::CGrabResultPtr *>(const_cast<void *>(p));
}
static Pylon::CDeviceInfo *devinfo_of(void *p)
{
    return static_cast<Pylon::CDeviceInfo *>(p);
}
static Pylon::CDeviceInfo *devinfo_of(const void *p)
{
    return static_cast<Pylon::CDeviceInfo *>(const_cast<void *>(p));
}
static GenApi::INodeMap *nodemap_of(void *p)
{
    return static_cast<GenApi::INodeMap *>(p);
}
static GenApi::INodeMap *nodemap_of(const void *p)
{
    return static_cast<GenApi::INodeMap *>(const_cast<void *>(p));
}

static Pylon::EGrabStrategy convert_grab_strategy(int s)
{
    switch (s) {
    case 0: return Pylon::GrabStrategy_OneByOne;
    case 1: return Pylon::GrabStrategy_LatestImageOnly;
    case 2: return Pylon::GrabStrategy_LatestImages;
    case 3: return Pylon::GrabStrategy_UpcomingImage;
    default: throw std::invalid_argument("unknown grab strategy");
    }
}

extern "C" {

/* -------------------------------------------------------------------------
 * Memory management
 * ---------------------------------------------------------------------- */

void pylon_cxx_free_str(char *s)
{
    free(s);
}

void pylon_cxx_free_ptr(void *p)
{
    free(p);
}

/* -------------------------------------------------------------------------
 * Pylon lifecycle
 * ---------------------------------------------------------------------- */

void pylon_initialize(void)
{
    Pylon::PylonInitialize();
}

void pylon_terminate(int shutdown_logging)
{
    Pylon::PylonTerminate(shutdown_logging != 0);
}

void pylon_get_version(uint32_t *major, uint32_t *minor,
                       uint32_t *subminor, uint32_t *build)
{
    Pylon::GetPylonVersion(major, minor, subminor, build);
}

/* -------------------------------------------------------------------------
 * TlFactory
 * ---------------------------------------------------------------------- */

const char *tl_factory_create_first_device(void **out_camera)
{
    SHIM_TRY
    *out_camera = new Pylon::CInstantCamera(
        Pylon::CTlFactory::GetInstance().CreateFirstDevice());
    return nullptr;
    SHIM_CATCH
}

const char *tl_factory_create_device(const void *device_info,
                                     void **out_camera)
{
    SHIM_TRY
    *out_camera = new Pylon::CInstantCamera(
        Pylon::CTlFactory::GetInstance().CreateDevice(*devinfo_of(device_info)));
    return nullptr;
    SHIM_CATCH
}

const char *tl_factory_enumerate_devices(void ***out_infos, size_t *out_len)
{
    SHIM_TRY
    Pylon::DeviceInfoList_t devices;
    Pylon::CTlFactory::GetInstance().EnumerateDevices(devices);
    size_t n = devices.size();
    void **arr = (void **)malloc(n * sizeof(void *));
    for (size_t i = 0; i < n; i++) {
        arr[i] = new Pylon::CDeviceInfo(devices[i]);
    }
    *out_infos = arr;
    *out_len = n;
    return nullptr;
    SHIM_CATCH
}

/* -------------------------------------------------------------------------
 * CInstantCamera
 * ---------------------------------------------------------------------- */

void instant_camera_destroy(void *camera)
{
    delete cam_of(camera);
}

const char *instant_camera_get_device_info(const void *camera,
                                           void **out_device_info)
{
    SHIM_TRY
    *out_device_info = new Pylon::CDeviceInfo(cam_of(camera)->GetDeviceInfo());
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_open(void *camera)
{
    SHIM_TRY
    cam_of(camera)->Open();
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_is_open(const void *camera, int *out)
{
    SHIM_TRY
    *out = cam_of(camera)->IsOpen() ? 1 : 0;
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_close(void *camera)
{
    SHIM_TRY
    cam_of(camera)->Close();
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_start_grabbing(void *camera)
{
    SHIM_TRY
    cam_of(camera)->StartGrabbing();
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_stop_grabbing(void *camera)
{
    SHIM_TRY
    cam_of(camera)->StopGrabbing();
    return nullptr;
    SHIM_CATCH
}

int instant_camera_is_grabbing(const void *camera)
{
    return cam_of(camera)->IsGrabbing() ? 1 : 0;
}

const char *instant_camera_start_grabbing_with_strategy(void *camera,
                                                         int grab_strategy)
{
    SHIM_TRY
    cam_of(camera)->StartGrabbing(convert_grab_strategy(grab_strategy));
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_start_grabbing_with_count(void *camera,
                                                      uint32_t count)
{
    SHIM_TRY
    cam_of(camera)->StartGrabbing((size_t)count);
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_start_grabbing_with_count_and_strategy(
    void *camera, uint32_t count, int grab_strategy)
{
    SHIM_TRY
    cam_of(camera)->StartGrabbing((size_t)count,
                                  convert_grab_strategy(grab_strategy));
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_retrieve_result(void *camera,
                                           uint32_t timeout_ms,
                                           void *grab_result,
                                           int timeout_handling,
                                           int *out_grabbed)
{
    SHIM_TRY
    Pylon::ETimeoutHandling eth = (timeout_handling == 0)
        ? Pylon::TimeoutHandling_Return
        : Pylon::TimeoutHandling_ThrowException;
    bool grabbed = cam_of(camera)->RetrieveResult(
        timeout_ms, *grab_of(grab_result), eth);
    *out_grabbed = grabbed ? 1 : 0;
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_get_node_map(const void *camera,
                                        const void **out_node_map)
{
    SHIM_TRY
    *out_node_map = &cam_of(const_cast<void*>(camera))->GetNodeMap();
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_get_tl_node_map(const void *camera,
                                           const void **out_node_map)
{
    SHIM_TRY
    *out_node_map = &cam_of(const_cast<void*>(camera))->GetTLNodeMap();
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_get_stream_grabber_node_map(
    const void *camera, const void **out_node_map)
{
    SHIM_TRY
    *out_node_map = &cam_of(const_cast<void*>(camera))->GetStreamGrabberNodeMap();
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_get_event_grabber_node_map(
    const void *camera, const void **out_node_map)
{
    SHIM_TRY
    *out_node_map = &cam_of(const_cast<void*>(camera))->GetEventGrabberNodeMap();
    return nullptr;
    SHIM_CATCH
}

const char *instant_camera_get_instant_camera_node_map(
    const void *camera, const void **out_node_map)
{
    SHIM_TRY
    *out_node_map = &cam_of(const_cast<void*>(camera))->GetInstantCameraNodeMap();
    return nullptr;
    SHIM_CATCH
}

#if !defined(_WIN32)
const char *instant_camera_wait_object_fd(const void *camera, int *out_fd)
{
    SHIM_TRY
    *out_fd = cam_of(const_cast<void*>(camera))->GetGrabResultWaitObject().GetFd();
    return nullptr;
    SHIM_CATCH
}
#endif

#if defined(_WIN32)
const char *instant_camera_wait_object(const void *camera,
                                       void **out_wait_object)
{
    SHIM_TRY
    *out_wait_object = new Pylon::WaitObject(
        cam_of(camera)->GetGrabResultWaitObject());
    return nullptr;
    SHIM_CATCH
}
#endif

/* -------------------------------------------------------------------------
 * NodeMap operations
 * ---------------------------------------------------------------------- */

const char *node_map_load(const void *node_map, const char *filename,
                          size_t filename_len, int validate)
{
    SHIM_TRY
    std::string fn(filename, filename_len);
    Pylon::CFeaturePersistence::Load(
        fn.c_str(),
        const_cast<GenApi::INodeMap *>(nodemap_of(node_map)),
        validate != 0);
    return nullptr;
    SHIM_CATCH
}

const char *node_map_save(const void *node_map, const char *filename,
                          size_t filename_len)
{
    SHIM_TRY
    std::string fn(filename, filename_len);
    Pylon::CFeaturePersistence::Save(
        fn.c_str(),
        const_cast<GenApi::INodeMap *>(nodemap_of(node_map)));
    return nullptr;
    SHIM_CATCH
}

const char *node_map_load_from_string(const void *node_map,
                                      const char *features,
                                      size_t features_len, int validate)
{
    SHIM_TRY
    std::string feats(features, features_len);
    Pylon::CFeaturePersistence::LoadFromString(
        feats.c_str(),
        const_cast<GenApi::INodeMap *>(nodemap_of(node_map)),
        validate != 0);
    return nullptr;
    SHIM_CATCH
}

const char *node_map_save_to_string(const void *node_map, char **out_str)
{
    SHIM_TRY
    Pylon::String_t result;
    Pylon::CFeaturePersistence::SaveToString(
        result,
        const_cast<GenApi::INodeMap *>(nodemap_of(node_map)));
    *out_str = strdup(result.c_str());
    return nullptr;
    SHIM_CATCH
}

const char *node_map_get_boolean_parameter(const void *node_map,
                                           const char *name, size_t name_len,
                                           void **out)
{
    SHIM_TRY
    GenApi::INodeMap &nm = *const_cast<GenApi::INodeMap *>(nodemap_of(node_map));
    Pylon::String_t n(name, name_len);
    *out = new Pylon::CBooleanParameter(nm, n);
    return nullptr;
    SHIM_CATCH
}

const char *node_map_get_integer_parameter(const void *node_map,
                                           const char *name, size_t name_len,
                                           void **out)
{
    SHIM_TRY
    GenApi::INodeMap &nm = *const_cast<GenApi::INodeMap *>(nodemap_of(node_map));
    Pylon::String_t n(name, name_len);
    *out = new Pylon::CIntegerParameter(nm, n);
    return nullptr;
    SHIM_CATCH
}

const char *node_map_get_float_parameter(const void *node_map,
                                         const char *name, size_t name_len,
                                         void **out)
{
    SHIM_TRY
    GenApi::INodeMap &nm = *const_cast<GenApi::INodeMap *>(nodemap_of(node_map));
    Pylon::String_t n(name, name_len);
    *out = new Pylon::CFloatParameter(nm, n);
    return nullptr;
    SHIM_CATCH
}

const char *node_map_get_enum_parameter(const void *node_map,
                                        const char *name, size_t name_len,
                                        void **out)
{
    SHIM_TRY
    GenApi::INodeMap &nm = *const_cast<GenApi::INodeMap *>(nodemap_of(node_map));
    Pylon::String_t n(name, name_len);
    *out = new Pylon::CEnumParameter(nm, n);
    return nullptr;
    SHIM_CATCH
}

const char *node_map_get_command_parameter(const void *node_map,
                                           const char *name, size_t name_len,
                                           void **out)
{
    SHIM_TRY
    GenApi::INodeMap &nm = *const_cast<GenApi::INodeMap *>(nodemap_of(node_map));
    Pylon::String_t n(name, name_len);
    *out = new Pylon::CCommandParameter(nm, n);
    return nullptr;
    SHIM_CATCH
}

/* -------------------------------------------------------------------------
 * CBooleanParameter
 * ---------------------------------------------------------------------- */

void boolean_parameter_destroy(void *p)
{
    delete static_cast<Pylon::CBooleanParameter *>(p);
}

const char *boolean_node_get_value(const void *p, int *out)
{
    SHIM_TRY
    *out = static_cast<const Pylon::CBooleanParameter *>(p)->GetValue() ? 1 : 0;
    return nullptr;
    SHIM_CATCH
}

const char *boolean_node_set_value(void *p, int value)
{
    SHIM_TRY
    static_cast<Pylon::CBooleanParameter *>(p)->SetValue(value != 0);
    return nullptr;
    SHIM_CATCH
}

/* -------------------------------------------------------------------------
 * CIntegerParameter
 * ---------------------------------------------------------------------- */

void integer_parameter_destroy(void *p)
{
    delete static_cast<Pylon::CIntegerParameter *>(p);
}

const char *integer_node_get_unit(const void *p, char **out)
{
    SHIM_TRY
    Pylon::String_t u = static_cast<Pylon::CIntegerParameter *>(const_cast<void*>(p))->GetUnit();
    *out = strdup(u.c_str());
    return nullptr;
    SHIM_CATCH
}

const char *integer_node_get_value(const void *p, int64_t *out)
{
    SHIM_TRY
    *out = static_cast<Pylon::CIntegerParameter *>(const_cast<void*>(p))->GetValue();
    return nullptr;
    SHIM_CATCH
}

const char *integer_node_get_min(const void *p, int64_t *out)
{
    SHIM_TRY
    *out = static_cast<Pylon::CIntegerParameter *>(const_cast<void*>(p))->GetMin();
    return nullptr;
    SHIM_CATCH
}

const char *integer_node_get_max(const void *p, int64_t *out)
{
    SHIM_TRY
    *out = static_cast<Pylon::CIntegerParameter *>(const_cast<void*>(p))->GetMax();
    return nullptr;
    SHIM_CATCH
}

const char *integer_node_set_value(void *p, int64_t value)
{
    SHIM_TRY
    static_cast<Pylon::CIntegerParameter *>(p)->SetValue(value);
    return nullptr;
    SHIM_CATCH
}

/* -------------------------------------------------------------------------
 * CFloatParameter
 * ---------------------------------------------------------------------- */

void float_parameter_destroy(void *p)
{
    delete static_cast<Pylon::CFloatParameter *>(p);
}

const char *float_node_get_unit(const void *p, char **out)
{
    SHIM_TRY
    Pylon::String_t u = static_cast<Pylon::CFloatParameter *>(const_cast<void *>(p))->GetUnit();
    *out = strdup(u.c_str());
    return nullptr;
    SHIM_CATCH
}

const char *float_node_get_value(const void *p, double *out)
{
    SHIM_TRY
    *out = static_cast<Pylon::CFloatParameter *>(const_cast<void*>(p))->GetValue();
    return nullptr;
    SHIM_CATCH
}

const char *float_node_get_min(const void *p, double *out)
{
    SHIM_TRY
    *out = static_cast<Pylon::CFloatParameter *>(const_cast<void*>(p))->GetMin();
    return nullptr;
    SHIM_CATCH
}

const char *float_node_get_max(const void *p, double *out)
{
    SHIM_TRY
    *out = static_cast<Pylon::CFloatParameter *>(const_cast<void*>(p))->GetMax();
    return nullptr;
    SHIM_CATCH
}

const char *float_node_set_value(void *p, double value)
{
    SHIM_TRY
    static_cast<Pylon::CFloatParameter *>(p)->SetValue(value);
    return nullptr;
    SHIM_CATCH
}

/* -------------------------------------------------------------------------
 * CEnumParameter
 * ---------------------------------------------------------------------- */

void enum_parameter_destroy(void *p)
{
    delete static_cast<Pylon::CEnumParameter *>(p);
}

const char *enum_node_get_value(const void *p, char **out)
{
    SHIM_TRY
    Pylon::String_t v = static_cast<Pylon::CEnumParameter *>(const_cast<void*>(p))->GetValue();
    *out = strdup(v.c_str());
    return nullptr;
    SHIM_CATCH
}

const char *enum_node_settable_values(const void *p, char ***out_values,
                                      size_t *out_count)
{
    SHIM_TRY
    Pylon::StringList_t names;
    static_cast<Pylon::CEnumParameter *>(const_cast<void*>(p))->GetSettableValues(names);
    size_t n = names.size();
    char **arr = (char **)malloc(n * sizeof(char *));
    for (size_t i = 0; i < n; i++) {
        arr[i] = strdup(names[i].c_str());
    }
    *out_values = arr;
    *out_count = n;
    return nullptr;
    SHIM_CATCH
}

void enum_node_free_settable_values(char **values, size_t count)
{
    for (size_t i = 0; i < count; i++) {
        free(values[i]);
    }
    free(values);
}

const char *enum_node_set_value(void *p, const char *value, size_t value_len)
{
    SHIM_TRY
    Pylon::String_t v(value, value_len);
    static_cast<Pylon::CEnumParameter *>(p)->SetValue(v);
    return nullptr;
    SHIM_CATCH
}

/* -------------------------------------------------------------------------
 * CCommandParameter
 * ---------------------------------------------------------------------- */

void command_parameter_destroy(void *p)
{
    delete static_cast<Pylon::CCommandParameter *>(p);
}

const char *command_node_execute(void *p, int verify)
{
    SHIM_TRY
    static_cast<Pylon::CCommandParameter *>(p)->Execute(verify != 0);
    return nullptr;
    SHIM_CATCH
}

/* -------------------------------------------------------------------------
 * CGrabResultPtr
 * ---------------------------------------------------------------------- */

const char *new_grab_result_ptr(void **out)
{
    SHIM_TRY
    *out = new Pylon::CGrabResultPtr();
    return nullptr;
    SHIM_CATCH
}

void grab_result_ptr_destroy(void *p)
{
    delete grab_of(p);
}

const char *grab_result_grab_succeeded(const void *p, int *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GrabSucceeded() ? 1 : 0;
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_error_description(const void *p, char **out)
{
    SHIM_TRY
    Pylon::String_t desc = (*grab_of(p))->GetErrorDescription();
    *out = strdup(desc.c_str());
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_error_code(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetErrorCode();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_width(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetWidth();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_height(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetHeight();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_offset_x(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetOffsetX();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_offset_y(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetOffsetY();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_padding_x(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetPaddingX();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_padding_y(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetPaddingY();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_buffer(const void *p, const uint8_t **out_buf,
                               size_t *out_len)
{
    SHIM_TRY
    *out_buf = reinterpret_cast<const uint8_t *>((*grab_of(p))->GetBuffer());
    *out_len = (*grab_of(p))->GetImageSize();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_payload_size(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetPayloadSize();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_buffer_size(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetImageSize();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_block_id(const void *p, uint64_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetBlockID();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_time_stamp(const void *p, uint64_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetTimeStamp();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_stride(const void *p, size_t *out)
{
    SHIM_TRY
    bool ok = (*grab_of(p))->GetStride(*out);
    (void)ok;
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_image_size(const void *p, uint32_t *out)
{
    SHIM_TRY
    *out = (*grab_of(p))->GetImageSize();
    return nullptr;
    SHIM_CATCH
}

const char *grab_result_get_chunk_data_node_map(const void *p,
                                                const void **out_node_map)
{
    SHIM_TRY
    *out_node_map = &(*grab_of(p))->GetChunkDataNodeMap();
    return nullptr;
    SHIM_CATCH
}

/* -------------------------------------------------------------------------
 * CDeviceInfo
 * ---------------------------------------------------------------------- */

void device_info_destroy(void *p)
{
    delete devinfo_of(p);
}

const char *device_info_clone(const void *p, void **out)
{
    SHIM_TRY
    *out = new Pylon::CDeviceInfo(*devinfo_of(p));
    return nullptr;
    SHIM_CATCH
}

const char *device_info_get_property_names(const void *p,
                                           char ***out_names,
                                           size_t *out_count)
{
    SHIM_TRY
    Pylon::StringList_t names;
    devinfo_of(p)->GetPropertyNames(names);
    size_t n = names.size();
    char **arr = (char **)malloc(n * sizeof(char *));
    for (size_t i = 0; i < n; i++) {
        arr[i] = strdup(names[i].c_str());
    }
    *out_names = arr;
    *out_count = n;
    return nullptr;
    SHIM_CATCH
}

void device_info_free_property_names(char **names, size_t count)
{
    for (size_t i = 0; i < count; i++) {
        free(names[i]);
    }
    free(names);
}

const char *device_info_get_property_value(const void *p,
                                           const char *name, size_t name_len,
                                           char **out)
{
    SHIM_TRY
    Pylon::String_t n(name, name_len);
    Pylon::String_t result;
    bool ok = devinfo_of(p)->GetPropertyValue(n, result);
    if (!ok) {
        return make_error("property not found");
    }
    *out = strdup(result.c_str());
    return nullptr;
    SHIM_CATCH
}

const char *device_info_get_model_name(const void *p, char **out)
{
    SHIM_TRY
    Pylon::String_t name = devinfo_of(p)->GetModelName();
    *out = strdup(name.c_str());
    return nullptr;
    SHIM_CATCH
}

#if defined(_WIN32)
/* -------------------------------------------------------------------------
 * WaitObject (Windows stream)
 * ---------------------------------------------------------------------- */

void wait_object_destroy(void *p)
{
    delete static_cast<Pylon::WaitObject *>(p);
}

const char *wait_object_wait(const void *p, uint64_t timeout_ms, int *out)
{
    SHIM_TRY
    *out = static_cast<const Pylon::WaitObject *>(p)->Wait(timeout_ms) ? 1 : 0;
    return nullptr;
    SHIM_CATCH
}
#endif

static const pylon_cabi_api k_pylon_cabi_api = {
    PYLON_CABI_VERSION,
    sizeof(pylon_cabi_api),

    pylon_cxx_free_str,
    pylon_cxx_free_ptr,

    pylon_initialize,
    pylon_terminate,
    pylon_get_version,

    tl_factory_create_first_device,
    tl_factory_create_device,
    tl_factory_enumerate_devices,

    instant_camera_destroy,
    instant_camera_get_device_info,
    instant_camera_open,
    instant_camera_is_open,
    instant_camera_close,
    instant_camera_start_grabbing,
    instant_camera_stop_grabbing,
    instant_camera_is_grabbing,
    instant_camera_start_grabbing_with_strategy,
    instant_camera_start_grabbing_with_count,
    instant_camera_start_grabbing_with_count_and_strategy,
    instant_camera_retrieve_result,
    instant_camera_get_node_map,
    instant_camera_get_tl_node_map,
    instant_camera_get_stream_grabber_node_map,
    instant_camera_get_event_grabber_node_map,
    instant_camera_get_instant_camera_node_map,
#if !defined(_WIN32)
    instant_camera_wait_object_fd,
#endif
#if defined(_WIN32)
    instant_camera_wait_object,
#endif

    node_map_load,
    node_map_save,
    node_map_load_from_string,
    node_map_save_to_string,
    node_map_get_boolean_parameter,
    node_map_get_integer_parameter,
    node_map_get_float_parameter,
    node_map_get_enum_parameter,
    node_map_get_command_parameter,

    boolean_parameter_destroy,
    boolean_node_get_value,
    boolean_node_set_value,

    integer_parameter_destroy,
    integer_node_get_unit,
    integer_node_get_value,
    integer_node_get_min,
    integer_node_get_max,
    integer_node_set_value,

    float_parameter_destroy,
    float_node_get_unit,
    float_node_get_value,
    float_node_get_min,
    float_node_get_max,
    float_node_set_value,

    enum_parameter_destroy,
    enum_node_get_value,
    enum_node_settable_values,
    enum_node_free_settable_values,
    enum_node_set_value,

    command_parameter_destroy,
    command_node_execute,

    new_grab_result_ptr,
    grab_result_ptr_destroy,
    grab_result_grab_succeeded,
    grab_result_error_description,
    grab_result_error_code,
    grab_result_width,
    grab_result_height,
    grab_result_offset_x,
    grab_result_offset_y,
    grab_result_padding_x,
    grab_result_padding_y,
    grab_result_buffer,
    grab_result_payload_size,
    grab_result_buffer_size,
    grab_result_block_id,
    grab_result_time_stamp,
    grab_result_stride,
    grab_result_image_size,
    grab_result_get_chunk_data_node_map,

    device_info_destroy,
    device_info_clone,
    device_info_get_property_names,
    device_info_free_property_names,
    device_info_get_property_value,
    device_info_get_model_name,

#if defined(_WIN32)
    wait_object_destroy,
    wait_object_wait,
#endif
};

const pylon_cabi_api *pylon_cabi_get_api(void)
{
    return &k_pylon_cabi_api;
}

const pylon_shim_api *pylon_shim_get_api(void)
{
    return pylon_cabi_get_api();
}

} /* extern "C" */
