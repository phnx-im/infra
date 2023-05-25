// SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
typedef struct _Dart_Handle* Dart_Handle;

typedef struct DartCObject DartCObject;

typedef int64_t DartPort;

typedef bool (*DartPostCObjectFnType)(DartPort port_id, void *message);

typedef struct wire_MutexCorelibDartNotifier {
  const void *ptr;
} wire_MutexCorelibDartNotifier;

typedef struct wire_RustState {
  struct wire_MutexCorelibDartNotifier corelib;
} wire_RustState;

typedef struct wire_uint_8_list {
  uint8_t *ptr;
  int32_t len;
} wire_uint_8_list;

typedef struct wire_UuidBytes {
  struct wire_uint_8_list *bytes;
} wire_UuidBytes;

typedef struct DartCObject *WireSyncReturn;

void store_dart_post_cobject(DartPostCObjectFnType ptr);

Dart_Handle get_dart_object(uintptr_t ptr);

void drop_dart_object(uintptr_t ptr);

uintptr_t new_dart_opaque(Dart_Handle handle);

intptr_t init_frb_dart_api_dl(void *obj);

void wire_init_lib(int64_t port_);

void wire_initialize_backend__method__RustState(int64_t port_,
                                                struct wire_RustState *that,
                                                struct wire_uint_8_list *url);

void wire_create_user__method__RustState(int64_t port_,
                                         struct wire_RustState *that,
                                         struct wire_uint_8_list *username);

void wire_create_conversation__method__RustState(int64_t port_,
                                                 struct wire_RustState *that,
                                                 struct wire_uint_8_list *name);

void wire_get_conversations__method__RustState(int64_t port_, struct wire_RustState *that);

void wire_invite_user__method__RustState(int64_t port_,
                                         struct wire_RustState *that,
                                         struct wire_UuidBytes *conversation_id,
                                         struct wire_uint_8_list *username);

void wire_send_message__method__RustState(int64_t port_,
                                          struct wire_RustState *that,
                                          struct wire_UuidBytes *conversation_id,
                                          struct wire_uint_8_list *message);

void wire_get_messages__method__RustState(int64_t port_,
                                          struct wire_RustState *that,
                                          struct wire_UuidBytes *conversation_id,
                                          uintptr_t last_n);

void wire_get_clients__method__RustState(int64_t port_, struct wire_RustState *that);

void wire_register_stream__method__RustState(int64_t port_, struct wire_RustState *that);

void wire_fetch_messages__method__RustState(int64_t port_, struct wire_RustState *that);

struct wire_MutexCorelibDartNotifier new_MutexCorelibDartNotifier(void);

struct wire_RustState *new_box_autoadd_rust_state_0(void);

struct wire_UuidBytes *new_box_autoadd_uuid_bytes_0(void);

struct wire_uint_8_list *new_uint_8_list_0(int32_t len);

void drop_opaque_MutexCorelibDartNotifier(const void *ptr);

const void *share_opaque_MutexCorelibDartNotifier(const void *ptr);

void free_WireSyncReturn(WireSyncReturn ptr);

static int64_t dummy_method_to_enforce_bundling(void) {
    int64_t dummy_var = 0;
    dummy_var ^= ((int64_t) (void*) wire_init_lib);
    dummy_var ^= ((int64_t) (void*) wire_initialize_backend__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_create_user__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_create_conversation__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_get_conversations__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_invite_user__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_send_message__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_get_messages__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_get_clients__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_register_stream__method__RustState);
    dummy_var ^= ((int64_t) (void*) wire_fetch_messages__method__RustState);
    dummy_var ^= ((int64_t) (void*) new_MutexCorelibDartNotifier);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_rust_state_0);
    dummy_var ^= ((int64_t) (void*) new_box_autoadd_uuid_bytes_0);
    dummy_var ^= ((int64_t) (void*) new_uint_8_list_0);
    dummy_var ^= ((int64_t) (void*) drop_opaque_MutexCorelibDartNotifier);
    dummy_var ^= ((int64_t) (void*) share_opaque_MutexCorelibDartNotifier);
    dummy_var ^= ((int64_t) (void*) free_WireSyncReturn);
    dummy_var ^= ((int64_t) (void*) store_dart_post_cobject);
    dummy_var ^= ((int64_t) (void*) get_dart_object);
    dummy_var ^= ((int64_t) (void*) drop_dart_object);
    dummy_var ^= ((int64_t) (void*) new_dart_opaque);
    return dummy_var;
}
