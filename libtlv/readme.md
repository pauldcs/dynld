# Thread local variable support
This library contains the functions necessary to enable TLS for an executable.
All of this code was copied directly from dyld.
While these functions require the standard library, and we cannot link to it, there is a little hack to allow them to be undefined at link time. We compile the dynamic linker with:
```
-Wl,-U,_malloc
-Wl,-U,_free
-Wl,-U,_abort
-Wl,-U,_pthread_key_create
-Wl,-U,_pthread_getspecific
-Wl,-U,_pthread_setspecific
-Wl,-U,_pthread_mutex_lock
-Wl,-U,_pthread_mutex_unlock
-Wl,-not_for_dyld_shared_cache
```
And when the dynamic linker runs, we looks for those functions in the dyld shared cache and manually bind them to it before calling `tlv_initialize_descriptors_export`.