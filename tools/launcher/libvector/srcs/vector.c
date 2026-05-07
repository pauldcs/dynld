#include "vector.h"

/* Overrides the vector's internal length value.
 * Note: This only updates the stored length; it does not modify
 * or resize the underlying buffer.
 */
inline __attribute__((always_inline)) void
__vector_set_length_internal(vector *__this, size_t len)
{
	__this->_len = len;
}

/* Overrides the vector's internal element size value.
 * Note: This only updates the stored element size; it does not modify
 * or resize the underlying buffer.
 */
inline __attribute__((always_inline)) void
__vector_set_elem_size_internal(vector *__this, size_t elem_size)
{
	__this->_elem_size = elem_size;
}

/* Overrides the vector's internal capacity value.
 * Note: This only updates the stored capacity; it does not modify
 * or resize the underlying buffer.
 */
inline __attribute__((always_inline)) void
__vector_set_capacity_internal(vector *__this, size_t cap)
{
	__this->_capacity = cap;
}

/* Overrides the vector's internal pointer value.
 * Note: This only updates the stored pointer; it does not modify
 * or resize the underlying buffer.
 */
inline __attribute__((always_inline)) void
__vector_set_ptr_internal(vector *__this, void *ptr)
{
	__this->_ptr = ptr;
}

/* Overrides the vector's internal destructor function pointer.
 * Note: This only updates the stored pointer; it does not modify
 * or resize the underlying buffer.
 */
inline __attribute__((always_inline)) void
__vector_set_destructor_internal(
	vector *__this, void (*destructor)(void *)
)
{
	__this->_destructor = destructor;
}

/* Internal vector buffer allocation function. It uses the allocator provided
 * by the call to `vector_init`
 */
inline bool __vector_alloc_internal(vector *__this, size_t init_size)
{
	__this->_ptr = __this->_allocator.alloc(init_size);
	return (__this->_ptr != NULL);
}

/* Internal vector buffer reallocation function. It uses the allocator provided
 * by the call to `vector_init`
 */
inline bool __vector_realloc_internal(vector *__this, size_t new_size)
{
	void *pointer = __this->_allocator.alloc(new_size);
	if (unlikely(pointer == NULL)) {
		return (false);
	}

	(void)__memcpy(
		pointer,
		vector_first_to_ptr_unchecked(__this),
		__this->_len * vector_elem_size(__this)
	);

	__this->_allocator.release(
		vector_first_to_ptr_unchecked(__this)
	);

	__vector_set_ptr_internal(__this, pointer);
	__vector_set_capacity_internal(__this, new_size);

	return (true);
}

/* Creates a vector and adjusts its starting capacity to be at least
 * enough to hold `n` elements.
 * Note: This function assumes that the vector pointer `uninit_vec` is
 * uninitialized *BUT* valid. The pointer must point to a buffer that holds
 * at least `sizeof(vector)` bytes.
 */
bool vector_init(
	vector		  *uninit_vec,
	vector_allocator_t allocator,
	size_t		   elem_size,
	size_t		   init_cap,
	void (*destroy)(void *)
)
{
	size_t capacity =
		elem_size * (init_cap == 0 ? MIN_CAP : init_cap);

	(void)__memset(uninit_vec, 0x00, sizeof(vector));

	uninit_vec->_allocator = allocator;

	__vector_alloc_internal(uninit_vec, capacity);

	if (unlikely(
		    vector_first_to_ptr_unchecked(uninit_vec) == NULL
	    )) {
		return (false);
	}

	__vector_set_elem_size_internal(uninit_vec, elem_size);
	__vector_set_capacity_internal(uninit_vec, capacity);
	__vector_set_destructor_internal(uninit_vec, destroy);

	return (true);
}

/* Returns the number of elements contained in the vector.
 */
inline __attribute__((always_inline)) size_t
vector_length(const vector *__this)
{
	return (__this->_len);
}

/* Returns the number of bytes required for one element.
 */
inline __attribute__((always_inline)) size_t
vector_elem_size(const vector *__this)
{
	return (__this->_elem_size);
}

/* Returns the number of bytes currently allocated for the
 * internal buffer.
 */
inline __attribute__((always_inline)) size_t
vector_capacity(const vector *__this)
{
	return (__this->_capacity);
}

/* Applies the (assumed to be) provided destructor on the
 * element at `position`.
 * Note: No checks are done, it's up to the caller to make sure
 * `position` is within bounds, and that a destructor exists.
 */
inline __attribute__((always_inline)) void
vector_index_apply_destructor_unchecked(
	const vector *__this, size_t position
)
{
	__this->_destructor(
		vector_index_to_ptr_unchecked(__this, position)
	);
}

/* Applies the (assumed to be) provided destructor on the
 * elements in the range `start` -> `end`.
 * Note: No checks are done, it's up to the caller to make sure
 * `start` and `end are valid, and that a destructor exists.
 */
inline __attribute__((always_inline)) void
vector_apply_destructor_in_range_unchecked(
	const vector *__this, size_t start, size_t end
)
{
	size_t i = end - start;
	while (i--) {
		vector_index_apply_destructor_unchecked(
			__this, start + i
		);
	}
}

/* Returns true if the vector contains no elements.
 */
inline __attribute__((always_inline)) bool
vector_is_empty(const vector *__this)
{
	return (vector_length(__this) == 0);
}

/* Returns true if the vector has a destructor.
 */
inline __attribute__((always_inline)) bool
vector_has_destructor(const vector *__this)
{
	return (__this->_destructor != NULL);
}

/* Returns true if the vector cannot hold additional elements
 * without having to reallocate it's internal buffer.
 */
inline __attribute__((always_inline)) bool
vector_is_full(const vector *__this)
{
	return (vector_size_of(__this) == vector_capacity(__this));
}

/* Returns the number of bytes in used by the vector's internal buffer.
 * Note: The value returned only concerns the internal buffer, and not the
 * size of the vector struct itself.
 */
inline __attribute__((always_inline)) size_t
vector_size_of(const vector *__this)
{
	return (vector_length(__this) * vector_elem_size(__this));
}

/* Returns a pointer to the element at the specified position.
 * Note: Be cautious when holding pointers to the vector’s internal
 * buffer. Many vector operations may reallocate the buffer, which
 * invalidates any existing pointers and leaves them dangling.
 * Whenever possible, prefer storing indices instead of pointers, or
 * at least avoid keeping pointers until the vector has reached what
 * is expected to be its final size.
 */
inline __attribute__((always_inline)) void *
vector_index_to_ptr(const vector *__this, size_t position)
{
	if (likely(position < vector_length(__this))) {
		return (vector_index_to_ptr_unchecked(
			__this, position
		));
	}

	return (NULL);
}

/* Returns a pointer to the element at the specified position.
 * Note: Zero bound checks are done.
 * Note: Be cautious when holding pointers to the vector’s internal
 * buffer. Many vector operations may reallocate the buffer, which
 * invalidates any existing pointers and leaves them dangling.
 * Whenever possible, prefer storing indices instead of pointers, or
 * at least avoid keeping pointers until the vector has reached what
 * is expected to be its final size.
 */
inline __attribute__((always_inline)) void *
vector_index_to_ptr_unchecked(const vector *__this, size_t position)
{
	return (
		((char *)vector_first_to_ptr_unchecked(__this) +
		 vector_elem_size(__this) * position)
	);
}

/* Returns the first element of the vector.
 * Note: Be cautious when holding pointers to the vector’s internal
 * buffer. Many vector operations may reallocate the buffer, which
 * invalidates any existing pointers and leaves them dangling.
 * Whenever possible, prefer storing indices instead of pointers, or
 * at least avoid keeping pointers until the vector has reached what
 * is expected to be its final size.
 */
inline __attribute__((always_inline)) void *
vector_first_to_ptr(const vector *__this)
{
	if (likely(!vector_is_empty(__this))) {
		return (vector_first_to_ptr_unchecked(__this));
	}

	return (NULL);
}

/* Returns the first element of the vector.
 * Note: Zero bound checks are done.
 * Note: Be cautious when holding pointers to the vector’s internal
 * buffer. Many vector operations may reallocate the buffer, which
 * invalidates any existing pointers and leaves them dangling.
 * Whenever possible, prefer storing indices instead of pointers, or
 * at least avoid keeping pointers until the vector has reached what
 * is expected to be its final size.
 */
inline __attribute__((always_inline)) void *
vector_first_to_ptr_unchecked(const vector *__this)
{
	return (__this->_ptr);
}

/* Returns the last element of the vector.
 * Note: Be cautious when holding pointers to the vector’s internal
 * buffer. Many vector operations may reallocate the buffer, which
 * invalidates any existing pointers and leaves them dangling.
 * Whenever possible, prefer storing indices instead of pointers, or
 * at least avoid keeping pointers until the vector has reached what
 * is expected to be its final size.
 */
inline __attribute__((always_inline)) void *
vector_last_to_ptr(const vector *__this)
{
	if (likely(!vector_is_empty(__this))) {
		return (vector_last_to_ptr_unchecked(__this));
	}

	return (NULL);
}

/* Returns the last element of the vector.
 * Note: Zero bound checks are done.
 * Note: Be cautious when holding pointers to the vector’s internal
 * buffer. Many vector operations may reallocate the buffer, which
 * invalidates any existing pointers and leaves them dangling.
 * Whenever possible, prefer storing indices instead of pointers, or
 * at least avoid keeping pointers until the vector has reached what
 * is expected to be its final size.
 */
inline __attribute__((always_inline)) void *
vector_last_to_ptr_unchecked(const vector *__this)
{
	return (vector_index_to_ptr_unchecked(
		__this, vector_length(__this) - 1
	));
}

/* Removes all the elements from the vector.
 * If a destructor was provided on initialization, it is applied to each
 * elements.
 * Note: The capacity will remain unchanged.
 */
inline void vector_clear(vector *__this)
{
	if (vector_has_destructor(__this)) {
		vector_apply_destructor_in_range_unchecked(
			__this, 0, vector_length(__this)
		);
	}

	__vector_set_length_internal(__this, 0);
}

/* Frees the vector internal buffer, clearing the content beforhand.
 * This function internally calls `vector_clear`, and then releases both
 * the vectors internal buffer. `__this` *MUST* be considered invalid
 * after having called this function on it. Though, it can be safely
 * passed to `vector_init` to initialize it again.
 */
inline void vector_deinit(vector *__this)
{
	vector_clear(__this);
	__this->_allocator.release(
		vector_first_to_ptr_unchecked(__this)
	);
}

/* Frees the vector internal buffer, clearing the content and zeroizing
 * the buffer beforhand.
 * This function internally calls `vector_clear`, and then releases both
 * the vectors internal buffer. `__this` *MUST* be considered invalid
 * after having called this function on it. Though, it can be safely
 * passed to `vector_init` to initialize it again.
 */
inline void vector_deinit_zeroized(vector *__this)
{
	if (likely(vector_capacity(__this))) {
		__memset(
			vector_first_to_ptr_unchecked(__this),
			0,
			vector_capacity(__this)
		);
	}

	__this->_allocator.release(
		vector_first_to_ptr_unchecked(__this)
	);
}

/* Adjusts the vector capacity to be at least enough to
 * contain an additional `n` elements.
 */
bool vector_adjust_cap_if_full(vector *__this, size_t n)
{
	n += vector_length(__this);
	n *= vector_elem_size(__this);

	if (likely(n < vector_capacity(__this))) {
		return (true);
	}

	size_t twice_capacity = vector_capacity(__this) * 2;
	if (twice_capacity < MIN_CAP) {
		twice_capacity = MIN_CAP;
	}

	return (
		likely(__vector_realloc_internal(
			       __this,
			       n > twice_capacity ? n : twice_capacity
		       ) != false)
	);
}

/* Adjusts the vector capacity to have the exact amount of capacity
 * to hold an additional 'n' elements *UNLESS* enough capacity already
 * exists. No truncation is done in that case and the buffer remains
 * unchanged.
 */
inline bool vector_adjust_exact_cap_if_full(vector *__this, size_t n)
{
	n += vector_length(__this);
	n *= vector_elem_size(__this);

	if (likely(n < vector_capacity(__this))) {
		return (true);
	}

	return (likely(__vector_realloc_internal(__this, n)));
}

/* Adds a new element at the end of the vector, after its current
 * last element. The data pointed to by `e` is copied to the
 * new element.
 * Note: internally calls `memmove` with the pointer provided. The
 * pointer itself is not copied but the value pointed by it.
 */
inline bool vector_push(vector *__this, const void *element)
{
	if (unlikely(!vector_adjust_cap_if_full(__this, 1))) {
		return (false);
	}

	vector_push_within_inner_unchecked(__this, element);

	return (true);
}

/* Adds a new element at the end of the vector, after its current
 * last element. The data pointed to by 'e' is copied to the
 * new element.
 * Note: The function returns false if the internal buffer must be
 * reallocated. Calling this function does not invalidate existing
 * pointers to elements within the buffer.
 */
inline bool
vector_push_within_inner(vector *__this, const void *element)
{
	if (unlikely(
		    vector_size_of(__this) +
			    vector_elem_size(__this) * 1 >
		    vector_capacity(__this)
	    )) {
		return (false);
	}

	vector_push_within_inner_unchecked(__this, element);

	return (true);
}

/* Adds a new element at the end of the vector, after its current
 * last element. The data pointed to by 'e' is copied to the
 * new element.
 * Note: No bound checks are done by this function.
 */
inline void
vector_push_within_inner_unchecked(vector *__this, const void *element)
{
	(void)__memmove(
		vector_uninitialized_data(__this),
		element,
		vector_elem_size(__this)
	);

	vector_append_from_capacity(__this, 1);
}

/* Removes the last element of the vector, effectively reducing
 * the container size by one.
 */
inline void vector_pop(vector *__this)
{
	if (unlikely(vector_length(__this) == 0)) {
		return;
	}

	if (vector_has_destructor(__this)) {
		vector_index_apply_destructor_unchecked(
			__this, vector_length(__this) - 1
		);
	}

	__vector_set_length_internal(
		__this, vector_length(__this) - 1
	);
}

/* The vector is extended by injecting a new element before the
 * element at the specified position, effectively increasing
 * the vector's size by one. Inserting in the vector can only happen
 * *BEFORE* and existing element, so the vector must have at least one
 * element for this method to work.
 * Insertion is permitted only before an existing element. Consequently,
 * this method requires the vector to contain at least one item.
 */
inline bool
vector_insert(vector *__this, size_t position, const void *element)
{
	if (unlikely(position >= vector_length(__this)) ||
	    unlikely(!vector_adjust_cap_if_full(__this, 1))) {
		return (false);
	}

	vector_insert_within_inner_unchecked(
		__this, position, element
	);

	return (true);
}

/* The vector is extended by injecting a new element before the
 * element at the specified position, effectively increasing the vector's size
 * by one.
 * Insertion is permitted only before an existing element. Consequently,
 * this method requires the vector to contain at least one item.
 * Note: The function returns false if the internal buffer must be
 * reallocated. Calling this function does not invalidate existing
 * pointers to elements within the buffer.
 */
inline bool vector_insert_within_inner(
	vector *__this, size_t position, const void *element
)
{
	if (unlikely(position >= vector_length(__this)) ||
	    unlikely(
		    (vector_size_of(__this) +
		     vector_elem_size(__this) * 1) >
		    vector_capacity(__this)
	    )) {
		return (false);
	}

	vector_insert_within_inner_unchecked(
		__this, position, element
	);

	return (true);
}

/* The vector is extended by injecting a new element before the
 * element at the specified position, effectively increasing the vector's size
 * by one.
 * Insertion is permitted only before an existing element. Consequently,
 * this method requires the vector to contain at least one item.
 * Note: No bound checks are done by this function.
 */
inline void vector_insert_within_inner_unchecked(
	vector *__this, size_t position, const void *element
)
{
	(void)__memmove(
		(char *)vector_index_to_ptr_unchecked(
			__this, position + 1
		),
		(char *)vector_index_to_ptr_unchecked(
			__this, position
		),
		vector_size_of(__this) -
			position * vector_elem_size(__this)
	);

	(void)__memcpy(
		(char *)vector_index_to_ptr_unchecked(
			__this, position
		),
		element,
		vector_elem_size(__this)
	);

	vector_append_from_capacity(__this, 1);
}

/* Injects 'n' elements pointed to by 'src' into the vector, at
 * potitions 'p'.
 */
inline bool vector_copy_contiguous(
	vector *__this, size_t position, const void *src, size_t len
)
{
	if (unlikely(position > vector_length(__this)) ||
	    unlikely(!vector_adjust_cap_if_full(__this, len))) {
		return (false);
	}

	vector_copy_contiguous_within_inner_unchecked(
		__this, position, src, len
	);

	return (true);
}

/* Injects `n` elements pointed to by `src` into the vector, at
 * potitions `p`.
 * Note: The function returns false if the internal buffer must be
 * reallocated. Calling this function does not invalidate existing
 * pointers to elements within the buffer.
 */
bool vector_copy_contiguous_within_inner(
	vector *__this, size_t position, const void *src, size_t len
)
{
	if (unlikely(position > vector_length(__this)) ||
	    unlikely(
		    (vector_size_of(__this) +
		     vector_elem_size(__this) * len) >
		    vector_capacity(__this)
	    )) {
		return (false);
	}

	vector_copy_contiguous_within_inner_unchecked(
		__this, position, src, len
	);

	return (true);
}

/* Injects `n` elements pointed to by `src` into the vector, at
 * potitions `p`.
 * Note: No bound checks are done by this function.
 */
void vector_copy_contiguous_within_inner_unchecked(
	vector *__this, size_t position, const void *src, size_t len
)
{
	if (position < vector_length(__this)) {
		(void)__memmove(
			(char *)vector_index_to_ptr_unchecked(
				__this, position + len
			),
			(char *)vector_index_to_ptr_unchecked(
				__this, position
			),
			vector_elem_size(__this) *
				(vector_length(__this) - position)
		);
	}

	(void)__memmove(
		(char *)vector_index_to_ptr_unchecked(
			__this, position
		),
		src,
		vector_elem_size(__this) * len
	);

	vector_append_from_capacity(__this, len);
}

/* Adds a new element to the front of the vector, before the
 * first element. The content of `element` is copied by internally
 * calling `vector_insert`
 */
inline __attribute__((always_inline)) bool
vector_pushf(vector *__this, const void *element)
{
	return (vector_insert(__this, 0, element));
}

/* Adds a new element to the front of the vector, before the
 * first element. The content of `element` is copied by internally
 * calling `vector_pushf_within_inner`
 */
inline __attribute__((always_inline)) bool
vector_pushf_within_inner(vector *__this, const void *element)
{
	return (vector_insert_within_inner(__this, 0, element));
}

/* Adds a new element to the front of the vector, before the
 * first element. The content of `element` is copied by internally
 * calling `vector_pushf_within_inner`.
 * Note: No bound checks are done by this function.
 */
inline __attribute__((always_inline)) void
vector_pushf_within_inner_unchecked(
	vector *__this, const void *element
)
{
	vector_insert_within_inner_unchecked(__this, 0, element);
}

/* Removes the first element from the vector, reducing
 * the container size by one.
 */
inline __attribute__((always_inline)) void vector_popf(vector *__this)
{
	vector_remove(__this, 0);
}

/* Removes the element at position `position` from the vector,
 * decreasing the size by one.
 */
inline void vector_remove(vector *__this, size_t position)
{
	if (unlikely(position >= vector_length(__this))) {
		return;
	}

	if (vector_has_destructor(__this)) {
		vector_index_apply_destructor_unchecked(
			__this, position
		);
	}

	vector_leak_unchecked(__this, position);
}

/* Removes `len` elements from the vector starting at index `start`,
 * leaving the vector with a length of vector_length() - len.
 */
inline void
vector_remove_range(vector *__this, size_t start, size_t len)
{
	if (start >= vector_length(__this) ||
	    start + len >= vector_length(__this)) {
		return;
	}

	if (vector_has_destructor(__this)) {
		vector_apply_destructor_in_range_unchecked(
			__this, 0, len
		);
	}

	vector_leak_range_unchecked(__this, start, len);
}

/* Removes the element at position `position` from the vector, while
 * skipping using the destructor, decreasing the size by one.
 */
inline void vector_leak(vector *__this, size_t position)
{
	if (unlikely(position >= vector_length(__this))) {
		return;
	}

	vector_leak_unchecked(__this, position);
}

/* Removes the element at position `position` from the vector, while
 * skipping using the destructor, decreasing the size by one.
 * Note: no bound checks are done.
 */
inline void vector_leak_unchecked(vector *__this, size_t position)
{
	size_t n = (vector_length(__this) - position) *
		   vector_elem_size(__this);

	if (likely(position <= vector_length(__this))) {
		(void)__memmove(
			vector_index_to_ptr_unchecked(
				__this, position
			),
			vector_index_to_ptr_unchecked(
				__this, position + 1
			),
			n - vector_elem_size(__this)
		);
	}

	__vector_set_length_internal(
		__this, vector_length(__this) - 1
	);
}

/* Removes `len` elements from the vector starting at index `start`,
 * leaving the vector with a length of vector_length() - len.
 * Note: the element destructor is not applied.
 */
inline void
vector_leak_range(vector *__this, size_t start, size_t len)
{
	if (start >= vector_length(__this) ||
	    start + len >= vector_length(__this)) {
		return;
	}

	vector_leak_range_unchecked(__this, start, len);
}

/* Removes `len` elements from the vector starting at index `start`,
 * leaving the vector with a length of vector_length() - len.
 * Note: the element destructor is not applied.
 * Note: no bound checks are done.
 */
inline void
vector_leak_range_unchecked(vector *__this, size_t start, size_t len)
{
	(void)__memmove(
		vector_index_to_ptr_unchecked(__this, start),
		vector_index_to_ptr_unchecked(__this, start + len),
		(vector_length(__this) - (start + len)) *
			vector_elem_size(__this)
	);

	__vector_set_length_internal(
		__this, vector_length(__this) - 1
	);
}

/* The element at position `a` and the element at position `b`
 * are swapped.
 */
inline void vector_swap_elems(vector *__this, size_t a, size_t b)
{
	size_t n = vector_elem_size(__this);

	char *p = vector_index_to_ptr_unchecked(__this, a);
	char *q = vector_index_to_ptr_unchecked(__this, b);

	for (; n--; ++p, ++q) {
		*p ^= *q;
		*q ^= *p;
		*p ^= *q;
	}
}

/* Reallocates the vector internal buffer if more than half of the current
 * capacity is currently unused.
 * If this function returns false, the initial vector remains untouched.
 */
inline bool vector_shrink_to_fit(vector *__this)
{
	if (vector_capacity(__this)) {
		size_t size = vector_size_of(__this);

		if (size < vector_capacity(__this) / 2) {
			if (!__vector_realloc_internal(__this, size)) {
				return (false);
			}
		}
	}

	return (true);
}

/* Creates a vector directly from a pointer, a length, a capacity etc.
 * Nothing is computed by the vector itself, the values are simply copied
 * into the fresh vector.
 * Note: While providing an optimized way to build a vector, this is very
 * dangerous due to the number of invariants that aren’t checked. In fact
 * nothing is checked, it assumes the caller carefully checked that everything
 * is valid.
 */
inline void vector_from_raw_parts(
	vector		  *uninit_vec,
	vector_allocator_t allocator,
	void		  *ptr,
	size_t		   elem_size,
	size_t		   len,
	size_t		   capacity,
	void (*destructor)(void *)
)
{
	__vector_set_capacity_internal(uninit_vec, capacity);
	__vector_set_length_internal(uninit_vec, len);
	__vector_set_ptr_internal(uninit_vec, ptr);
	__vector_set_elem_size_internal(uninit_vec, elem_size);
	uninit_vec->_allocator	= allocator;
	uninit_vec->_destructor = destructor;
}

/* Returns the position of an element given a pointer to it.
 * Note: This is highly dangerous, as nothing checks that the
 * pointer is in fact pointing to an element in the internal buffer.
 */
inline __attribute__((always_inline)) size_t
vector_elem_get_offset(const vector *__this, const void *element)
{
	return ((uintptr_t *)element -
		(uintptr_t *)vector_first_to_ptr_unchecked(__this));
}

/* Returns a ointer to the first uninitialized element in the buffer.
 * This function essencially returns a pointer to the the position
 * `vector_length() + 1`.
 * Note: This is very dangerous as the caller must ensure that the
 * memory at this position is within the capacity.
 * This can be done with a call to `vector_uninitialized_length()`
 */
inline __attribute__((always_inline)) void *
vector_uninitialized_data(const vector *__this)
{
	return (vector_index_to_ptr_unchecked(
		__this, vector_length(__this)
	));
}

/* Returns the number of elements that can fit in the buffer without
 * having to reallocate.
 */
inline size_t vector_uninitialized_length(const vector *__this)
{
	size_t size_in_bytes =
		vector_capacity(__this) - vector_size_of(__this);

	if (size_in_bytes) {
		size_in_bytes /= vector_elem_size(__this);
	}

	return (size_in_bytes);
}

/* Returns the number of uninitialized bytes allocated. This is similar
 * to `vector_uninitialized_length` while returning the number of actual
 * bytes.
 */
inline __attribute__((always_inline)) size_t
vector_uninitialized_size_of(const vector *__this)
{
	return (vector_capacity(__this) - vector_size_of(__this));
}

/* Appends `n` elements from capacity.
 * Note: This is very dangerous, the caller must ensure that the
 * uninitialized memory after `vector_length()` was manually initialized.
 * This function should only to be used with extra care in performance
 * critical context. This essencially only adds `n` to the vector length.
 */
inline __attribute__((always_inline)) void
vector_append_from_capacity(vector *__this, size_t n)
{
	__this->_len += n;
}
