export class IntoUnderlyingByteSource {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get autoAllocateChunkSize() {
        const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
    /**
     * @param {ReadableByteStreamController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    /**
     * @param {ReadableByteStreamController} controller
     */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, addHeapObject(controller));
    }
    /**
     * @returns {ReadableStreamType}
     */
    get type() {
        const ret = wasm.intounderlyingbytesource_type(this.__wbg_ptr);
        return __wbindgen_enum_ReadableStreamType[ret];
    }
}
if (Symbol.dispose) IntoUnderlyingByteSource.prototype[Symbol.dispose] = IntoUnderlyingByteSource.prototype.free;

export class IntoUnderlyingSink {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr, 0);
    }
    /**
     * @param {any} reason
     * @returns {Promise<any>}
     */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, addHeapObject(reason));
        return takeObject(ret);
    }
    /**
     * @returns {Promise<any>}
     */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return takeObject(ret);
    }
    /**
     * @param {any} chunk
     * @returns {Promise<any>}
     */
    write(chunk) {
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, addHeapObject(chunk));
        return takeObject(ret);
    }
}
if (Symbol.dispose) IntoUnderlyingSink.prototype[Symbol.dispose] = IntoUnderlyingSink.prototype.free;

export class IntoUnderlyingSource {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr, 0);
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
    /**
     * @param {ReadableStreamDefaultController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
}
if (Symbol.dispose) IntoUnderlyingSource.prototype[Symbol.dispose] = IntoUnderlyingSource.prototype.free;

/**
 * @param {string} filename
 * @param {Uint8Array} bytes
 */
export function import_web_epub(filename, bytes) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(filename, wasm.__wbindgen_export, wasm.__wbindgen_export2);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(bytes, wasm.__wbindgen_export);
        const len1 = WASM_VECTOR_LEN;
        wasm.import_web_epub(retptr, ptr0, len0, ptr1, len1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * @param {string} key
 * @param {Uint8Array} bytes
 * @returns {boolean}
 */
export function seed_web_book(key, bytes) {
    const ptr0 = passStringToWasm0(key, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArray8ToWasm0(bytes, wasm.__wbindgen_export);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.seed_web_book(ptr0, len0, ptr1, len1);
    return ret !== 0;
}

/**
 * @param {string | null} [library_json]
 * @param {string | null} [config_json]
 */
export function seed_web_state(library_json, config_json) {
    var ptr0 = isLikeNone(library_json) ? 0 : passStringToWasm0(library_json, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    var len0 = WASM_VECTOR_LEN;
    var ptr1 = isLikeNone(config_json) ? 0 : passStringToWasm0(config_json, wasm.__wbindgen_export, wasm.__wbindgen_export2);
    var len1 = WASM_VECTOR_LEN;
    wasm.seed_web_state(ptr0, len0, ptr1, len1);
}

/**
 * @param {number} safe_top
 * @param {number} safe_bottom
 * @param {number} safe_left
 * @param {number} safe_right
 */
export function start_bookworm_web(safe_top, safe_bottom, safe_left, safe_right) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.start_bookworm_web(retptr, safe_top, safe_bottom, safe_left, safe_right);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * @param {number} top
 * @param {number} bottom
 * @param {number} left
 * @param {number} right
 */
export function update_web_safe_area(top, bottom, left, right) {
    wasm.update_web_safe_area(top, bottom, left, right);
}

/**
 * Entry point for web workers
 * @param {number} ptr
 */
export function wasm_thread_entry_point(ptr) {
    wasm.wasm_thread_entry_point(ptr);
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Window_e6de206fcec1f90b: function(arg0) {
            const ret = getObject(arg0).Window;
            return addHeapObject(ret);
        },
        __wbg_WorkerGlobalScope_5b1c3fa5eed8a42d: function(arg0) {
            const ret = getObject(arg0).WorkerGlobalScope;
            return addHeapObject(ret);
        },
        __wbg___wbindgen_boolean_get_7a12af2b3f899c5a: function(arg0) {
            const v = getObject(arg0);
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_0e68cf47c9cbd9b0: function(arg0, arg1) {
            const ret = debugString(getObject(arg1));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_fcda5e3902d732fe: function(arg0) {
            const ret = typeof(getObject(arg0)) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_5160b3e381865372: function(arg0) {
            const ret = getObject(arg0) === null;
            return ret;
        },
        __wbg___wbindgen_is_string_c4f7cb494a2a21f1: function(arg0) {
            const ret = typeof(getObject(arg0)) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_8c687d0b90d5b524: function(arg0) {
            const ret = getObject(arg0) === undefined;
            return ret;
        },
        __wbg___wbindgen_memory_3f8442e22540244f: function() {
            const ret = wasm.memory;
            return addHeapObject(ret);
        },
        __wbg___wbindgen_module_c2aeeb3b14bad631: function() {
            const ret = wasmModule;
            return addHeapObject(ret);
        },
        __wbg___wbindgen_number_get_1dc732b810cb937c: function(arg0, arg1) {
            const obj = getObject(arg1);
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_92ab86bb19cbc12f: function(arg0, arg1) {
            const obj = getObject(arg1);
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_5d9e815e6fdf150f: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_997e73d32238e655: function(arg0) {
            getObject(arg0)._wbg_cb_unref();
        },
        __wbg_activeElement_a40cd04e4e5cfaff: function(arg0) {
            const ret = getObject(arg0).activeElement;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_activeTexture_79f1f4662ee9ed93: function(arg0, arg1) {
            getObject(arg0).activeTexture(arg1 >>> 0);
        },
        __wbg_activeTexture_e6dac82eb23d9409: function(arg0, arg1) {
            getObject(arg0).activeTexture(arg1 >>> 0);
        },
        __wbg_actualBoundingBoxAscent_80618394295ea13b: function(arg0) {
            const ret = getObject(arg0).actualBoundingBoxAscent;
            return ret;
        },
        __wbg_actualBoundingBoxDescent_3858fa415faa805d: function(arg0) {
            const ret = getObject(arg0).actualBoundingBoxDescent;
            return ret;
        },
        __wbg_actualBoundingBoxLeft_d1502206d15d1c17: function(arg0) {
            const ret = getObject(arg0).actualBoundingBoxLeft;
            return ret;
        },
        __wbg_actualBoundingBoxRight_e6c5cb3a984406bf: function(arg0) {
            const ret = getObject(arg0).actualBoundingBoxRight;
            return ret;
        },
        __wbg_addEventListener_9534e2dc12044ee4: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
        }, arguments); },
        __wbg_altKey_c75bf4903a5db4c2: function(arg0) {
            const ret = getObject(arg0).altKey;
            return ret;
        },
        __wbg_altKey_fa381deee4b395cd: function(arg0) {
            const ret = getObject(arg0).altKey;
            return ret;
        },
        __wbg_appendChild_0a61ee9a9da4556f: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).appendChild(getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_arrayBuffer_432de2b072afe7fb: function(arg0) {
            const ret = getObject(arg0).arrayBuffer();
            return addHeapObject(ret);
        },
        __wbg_attachShader_74ecf95efc73ee58: function(arg0, arg1, arg2) {
            getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
        },
        __wbg_attachShader_bfb57af27e3c4a25: function(arg0, arg1, arg2) {
            getObject(arg0).attachShader(getObject(arg1), getObject(arg2));
        },
        __wbg_beginQuery_8c123fa7e7aec2a4: function(arg0, arg1, arg2) {
            getObject(arg0).beginQuery(arg1 >>> 0, getObject(arg2));
        },
        __wbg_beginRenderPass_0d07417115f8f7c8: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).beginRenderPass(getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_bindAttribLocation_601c8f14be6021f1: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).bindAttribLocation(getObject(arg1), arg2 >>> 0, getStringFromWasm0(arg3, arg4));
        },
        __wbg_bindAttribLocation_f97028966e63a9a0: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).bindAttribLocation(getObject(arg1), arg2 >>> 0, getStringFromWasm0(arg3, arg4));
        },
        __wbg_bindBufferRange_34928ebaef37fa71: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).bindBufferRange(arg1 >>> 0, arg2 >>> 0, getObject(arg3), arg4, arg5);
        },
        __wbg_bindBuffer_24fab2b14c509d6f: function(arg0, arg1, arg2) {
            getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindBuffer_f1f33e087b91687c: function(arg0, arg1, arg2) {
            getObject(arg0).bindBuffer(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindFramebuffer_6c6a9caf742c2cdb: function(arg0, arg1, arg2) {
            getObject(arg0).bindFramebuffer(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindFramebuffer_dc00cdd13fca80a8: function(arg0, arg1, arg2) {
            getObject(arg0).bindFramebuffer(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindRenderbuffer_27f21bc7608d07e3: function(arg0, arg1, arg2) {
            getObject(arg0).bindRenderbuffer(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindRenderbuffer_a5af5373faa4fb14: function(arg0, arg1, arg2) {
            getObject(arg0).bindRenderbuffer(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindSampler_06a184f24b4f473a: function(arg0, arg1, arg2) {
            getObject(arg0).bindSampler(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindTexture_7858cc136e95ec24: function(arg0, arg1, arg2) {
            getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindTexture_e8a625bf5d240b0f: function(arg0, arg1, arg2) {
            getObject(arg0).bindTexture(arg1 >>> 0, getObject(arg2));
        },
        __wbg_bindVertexArrayOES_c947f00135eda668: function(arg0, arg1) {
            getObject(arg0).bindVertexArrayOES(getObject(arg1));
        },
        __wbg_bindVertexArray_4687bb1e8b327fc9: function(arg0, arg1) {
            getObject(arg0).bindVertexArray(getObject(arg1));
        },
        __wbg_blendColor_25f3208af2a1a86f: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).blendColor(arg1, arg2, arg3, arg4);
        },
        __wbg_blendColor_7b3873c65a61b8a7: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).blendColor(arg1, arg2, arg3, arg4);
        },
        __wbg_blendEquationSeparate_7255a352fb614ccb: function(arg0, arg1, arg2) {
            getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_blendEquationSeparate_a63d5c1a833c1137: function(arg0, arg1, arg2) {
            getObject(arg0).blendEquationSeparate(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_blendEquation_1636f14afa7d37d0: function(arg0, arg1) {
            getObject(arg0).blendEquation(arg1 >>> 0);
        },
        __wbg_blendEquation_9a5ca6936a9438b3: function(arg0, arg1) {
            getObject(arg0).blendEquation(arg1 >>> 0);
        },
        __wbg_blendFuncSeparate_1afc51f9c96486b2: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_blendFuncSeparate_222d39ec0d6bb147: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).blendFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_blendFunc_041c4d7dc50ac8c9: function(arg0, arg1, arg2) {
            getObject(arg0).blendFunc(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_blendFunc_ad7d043180d4f86a: function(arg0, arg1, arg2) {
            getObject(arg0).blendFunc(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_blitFramebuffer_beb217dd67c23446: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            getObject(arg0).blitFramebuffer(arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0);
        },
        __wbg_blockSize_726f0afadf8c1f3f: function(arg0) {
            const ret = getObject(arg0).blockSize;
            return ret;
        },
        __wbg_blur_c712b137baa8b295: function() { return handleError(function (arg0) {
            getObject(arg0).blur();
        }, arguments); },
        __wbg_body_7d83a19bffb260db: function(arg0) {
            const ret = getObject(arg0).body;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_body_9ee03724e9ad0639: function(arg0) {
            const ret = getObject(arg0).body;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_bottom_89fa307d0439be2c: function(arg0) {
            const ret = getObject(arg0).bottom;
            return ret;
        },
        __wbg_bufferData_5d0d376bd14d58f3: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        },
        __wbg_bufferData_9ce5cb209386d279: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferData(arg1 >>> 0, arg2, arg3 >>> 0);
        },
        __wbg_bufferData_b58d3ee720ef8b13: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
        },
        __wbg_bufferData_c074912da5a8e8c2: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferData(arg1 >>> 0, getObject(arg2), arg3 >>> 0);
        },
        __wbg_bufferSubData_1210904a130da787: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferSubData(arg1 >>> 0, arg2, getObject(arg3));
        },
        __wbg_bufferSubData_bb3556faa77ce63a: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).bufferSubData(arg1 >>> 0, arg2, getObject(arg3));
        },
        __wbg_buffer_4a989bded7035f57: function(arg0) {
            const ret = getObject(arg0).buffer;
            return addHeapObject(ret);
        },
        __wbg_buffer_60eec204070b2802: function(arg0) {
            const ret = getObject(arg0).buffer;
            return addHeapObject(ret);
        },
        __wbg_button_f6aa74c65b739064: function(arg0) {
            const ret = getObject(arg0).button;
            return ret;
        },
        __wbg_byobRequest_f161fc37241dd3d4: function(arg0) {
            const ret = getObject(arg0).byobRequest;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_byteLength_0ddb1795e2c7e689: function(arg0) {
            const ret = getObject(arg0).byteLength;
            return ret;
        },
        __wbg_byteOffset_46eb015f52b6ad7d: function(arg0) {
            const ret = getObject(arg0).byteOffset;
            return ret;
        },
        __wbg_call_269c5566fbede3eb: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).call(getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_call_6bcf8d3e20937e46: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_call_c1ad1cb1b78e8130: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3), getObject(arg4));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_cancelAnimationFrame_4fc0f227c45abccf: function() { return handleError(function (arg0, arg1) {
            getObject(arg0).cancelAnimationFrame(arg1);
        }, arguments); },
        __wbg_cancel_c076350a7e8954be: function(arg0) {
            const ret = getObject(arg0).cancel();
            return addHeapObject(ret);
        },
        __wbg_cancelable_cd494c5a45eb2bed: function(arg0) {
            const ret = getObject(arg0).cancelable;
            return ret;
        },
        __wbg_clearBufferfv_5c3233abb46232dd: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearBufferfv(arg1 >>> 0, arg2, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_clearBufferiv_a6017e170f127402: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearBufferiv(arg1 >>> 0, arg2, getArrayI32FromWasm0(arg3, arg4));
        },
        __wbg_clearBufferuiv_a7ff44fb5a19e886: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearBufferuiv(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4));
        },
        __wbg_clearDepth_a4e9a96e0b87fadb: function(arg0, arg1) {
            getObject(arg0).clearDepth(arg1);
        },
        __wbg_clearDepth_ea9081c357d405be: function(arg0, arg1) {
            getObject(arg0).clearDepth(arg1);
        },
        __wbg_clearRect_c52daae603b565de: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).clearRect(arg1, arg2, arg3, arg4);
        },
        __wbg_clearStencil_8c03daa5a936cf97: function(arg0, arg1) {
            getObject(arg0).clearStencil(arg1);
        },
        __wbg_clearStencil_bd3508b2e4a055c8: function(arg0, arg1) {
            getObject(arg0).clearStencil(arg1);
        },
        __wbg_clear_55f5d5ab26fe80b4: function(arg0, arg1) {
            getObject(arg0).clear(arg1 >>> 0);
        },
        __wbg_clear_faad852996d5b5ca: function(arg0, arg1) {
            getObject(arg0).clear(arg1 >>> 0);
        },
        __wbg_clientHeight_8669994caeda015c: function(arg0) {
            const ret = getObject(arg0).clientHeight;
            return ret;
        },
        __wbg_clientWaitSync_1e222800321920a4: function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).clientWaitSync(getObject(arg1), arg2 >>> 0, arg3 >>> 0);
            return ret;
        },
        __wbg_clientWidth_669677d957e958c7: function(arg0) {
            const ret = getObject(arg0).clientWidth;
            return ret;
        },
        __wbg_clientX_997500aa23d483d1: function(arg0) {
            const ret = getObject(arg0).clientX;
            return ret;
        },
        __wbg_clientY_52b1b3958eb93e2d: function(arg0) {
            const ret = getObject(arg0).clientY;
            return ret;
        },
        __wbg_clipboardData_21dbd4d02fe62f91: function(arg0) {
            const ret = getObject(arg0).clipboardData;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_clipboard_e39061ae5482d3b8: function(arg0) {
            const ret = getObject(arg0).clipboard;
            return addHeapObject(ret);
        },
        __wbg_close_22882088c136df25: function() { return handleError(function (arg0) {
            getObject(arg0).close();
        }, arguments); },
        __wbg_close_8b609460dd26e367: function() { return handleError(function (arg0) {
            getObject(arg0).close();
        }, arguments); },
        __wbg_colorMask_dc6ca7d28d90ca79: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
        },
        __wbg_colorMask_ee5503a19adab893: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).colorMask(arg1 !== 0, arg2 !== 0, arg3 !== 0, arg4 !== 0);
        },
        __wbg_compileShader_cabeb4aa8cb42914: function(arg0, arg1) {
            getObject(arg0).compileShader(getObject(arg1));
        },
        __wbg_compileShader_dada15a718167c7f: function(arg0, arg1) {
            getObject(arg0).compileShader(getObject(arg1));
        },
        __wbg_compressedTexSubImage2D_08a73ef1b07eadd4: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
            getObject(arg0).compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, getObject(arg8));
        },
        __wbg_compressedTexSubImage2D_26483f478d06e5b6: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
            getObject(arg0).compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, getObject(arg8));
        },
        __wbg_compressedTexSubImage2D_fd0d06f029b00475: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).compressedTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8, arg9);
        },
        __wbg_compressedTexSubImage3D_916c6fa47a2c6ca4: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            getObject(arg0).compressedTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, getObject(arg10));
        },
        __wbg_compressedTexSubImage3D_ca08254972ab7956: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            getObject(arg0).compressedTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10, arg11);
        },
        __wbg_configure_f99faab92b5e744d: function() { return handleError(function (arg0, arg1) {
            getObject(arg0).configure(getObject(arg1));
        }, arguments); },
        __wbg_contentRect_d4183dcae16ce988: function(arg0) {
            const ret = getObject(arg0).contentRect;
            return addHeapObject(ret);
        },
        __wbg_copyBufferSubData_4649cb7afad601b3: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).copyBufferSubData(arg1 >>> 0, arg2 >>> 0, arg3, arg4, arg5);
        },
        __wbg_copyTexSubImage2D_b5dc78e744bb31d3: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
            getObject(arg0).copyTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
        },
        __wbg_copyTexSubImage2D_f1d2043ea68078f1: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
            getObject(arg0).copyTexSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
        },
        __wbg_copyTexSubImage3D_a6674efc4225de77: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).copyTexSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9);
        },
        __wbg_createBindGroupLayout_45e16e6a488ebf6b: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).createBindGroupLayout(getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_createBindGroup_ec4dc4d9e46e1104: function(arg0, arg1) {
            const ret = getObject(arg0).createBindGroup(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_createBuffer_77a9a8580e5123cc: function(arg0) {
            const ret = getObject(arg0).createBuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createBuffer_9f4e696755111e6a: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).createBuffer(getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_createBuffer_b4dd77c6c40a2e87: function(arg0) {
            const ret = getObject(arg0).createBuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createCommandEncoder_a5136f45baaba888: function(arg0, arg1) {
            const ret = getObject(arg0).createCommandEncoder(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_createElement_b9024dc5ba95ac27: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_createFramebuffer_aa440c781a6c7da2: function(arg0) {
            const ret = getObject(arg0).createFramebuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createFramebuffer_ac4ca0fc2cfc1bd1: function(arg0) {
            const ret = getObject(arg0).createFramebuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createObjectURL_7dadebfd232e594a: function() { return handleError(function (arg0, arg1) {
            const ret = URL.createObjectURL(getObject(arg1));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_createPipelineLayout_a955ff1e35a63634: function(arg0, arg1) {
            const ret = getObject(arg0).createPipelineLayout(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_createProgram_89a2bb21dab7e222: function(arg0) {
            const ret = getObject(arg0).createProgram();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createProgram_e43ace4085d1086d: function(arg0) {
            const ret = getObject(arg0).createProgram();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createQuery_731629e292b35522: function(arg0) {
            const ret = getObject(arg0).createQuery();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createRenderPipeline_5147343dc33b1e18: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).createRenderPipeline(getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_createRenderbuffer_3605e09d07f5a422: function(arg0) {
            const ret = getObject(arg0).createRenderbuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createRenderbuffer_42eeb481e4c92ff2: function(arg0) {
            const ret = getObject(arg0).createRenderbuffer();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createSampler_2fe8901be471a83a: function(arg0, arg1) {
            const ret = getObject(arg0).createSampler(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_createSampler_ebfb34feb26878f1: function(arg0) {
            const ret = getObject(arg0).createSampler();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createShaderModule_f540847678b68e08: function(arg0, arg1) {
            const ret = getObject(arg0).createShaderModule(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_createShader_4d944e14daceede8: function(arg0, arg1) {
            const ret = getObject(arg0).createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createShader_70a84327b207bda4: function(arg0, arg1) {
            const ret = getObject(arg0).createShader(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createTexture_5e05f1b82904ab79: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).createTexture(getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_createTexture_a409a8c7972e99b3: function(arg0) {
            const ret = getObject(arg0).createTexture();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createTexture_ecccbb9df64e8c81: function(arg0) {
            const ret = getObject(arg0).createTexture();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createVertexArrayOES_083ec6fc6262613d: function(arg0) {
            const ret = getObject(arg0).createVertexArrayOES();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createVertexArray_9411e94d2ee6864e: function(arg0) {
            const ret = getObject(arg0).createVertexArray();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_createView_e5524224d95f3815: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).createView(getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_ctrlKey_785e802b8798af13: function(arg0) {
            const ret = getObject(arg0).ctrlKey;
            return ret;
        },
        __wbg_ctrlKey_ba07ac4b2ea90600: function(arg0) {
            const ret = getObject(arg0).ctrlKey;
            return ret;
        },
        __wbg_cullFace_3e14866a0c370004: function(arg0, arg1) {
            getObject(arg0).cullFace(arg1 >>> 0);
        },
        __wbg_cullFace_b64f84debd112644: function(arg0, arg1) {
            getObject(arg0).cullFace(arg1 >>> 0);
        },
        __wbg_data_3634610ec0d56ca6: function(arg0, arg1) {
            const ret = getObject(arg1).data;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_data_aa7ba49b34c0d883: function(arg0) {
            const ret = getObject(arg0).data;
            return addHeapObject(ret);
        },
        __wbg_data_efdd49457735dfbd: function(arg0, arg1) {
            const ret = getObject(arg1).data;
            const ptr1 = passArray8ToWasm0(ret, wasm.__wbindgen_export);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_debug_eb092a10b3338acb: function(arg0) {
            console.debug(getObject(arg0));
        },
        __wbg_deleteBook_732a44b255e49189: function(arg0, arg1) {
            window.__bookwormBridge.deleteBook(getStringFromWasm0(arg0, arg1));
        },
        __wbg_deleteBuffer_54e287dbecb17d6e: function(arg0, arg1) {
            getObject(arg0).deleteBuffer(getObject(arg1));
        },
        __wbg_deleteBuffer_c9dc22cdbb93ffbc: function(arg0, arg1) {
            getObject(arg0).deleteBuffer(getObject(arg1));
        },
        __wbg_deleteFramebuffer_6c3b62109b65e8de: function(arg0, arg1) {
            getObject(arg0).deleteFramebuffer(getObject(arg1));
        },
        __wbg_deleteFramebuffer_977fae88c7df1c6f: function(arg0, arg1) {
            getObject(arg0).deleteFramebuffer(getObject(arg1));
        },
        __wbg_deleteProgram_16fe8b000141443c: function(arg0, arg1) {
            getObject(arg0).deleteProgram(getObject(arg1));
        },
        __wbg_deleteProgram_c990794adb74a326: function(arg0, arg1) {
            getObject(arg0).deleteProgram(getObject(arg1));
        },
        __wbg_deleteQuery_670219eac170c9fb: function(arg0, arg1) {
            getObject(arg0).deleteQuery(getObject(arg1));
        },
        __wbg_deleteRenderbuffer_1066bd086097d9c0: function(arg0, arg1) {
            getObject(arg0).deleteRenderbuffer(getObject(arg1));
        },
        __wbg_deleteRenderbuffer_be22a49b59807a79: function(arg0, arg1) {
            getObject(arg0).deleteRenderbuffer(getObject(arg1));
        },
        __wbg_deleteSampler_a7f5bf9c8096d80a: function(arg0, arg1) {
            getObject(arg0).deleteSampler(getObject(arg1));
        },
        __wbg_deleteShader_7169f1760153b284: function(arg0, arg1) {
            getObject(arg0).deleteShader(getObject(arg1));
        },
        __wbg_deleteShader_9b34ccd212c66de7: function(arg0, arg1) {
            getObject(arg0).deleteShader(getObject(arg1));
        },
        __wbg_deleteSync_5f19ce0564df8ade: function(arg0, arg1) {
            getObject(arg0).deleteSync(getObject(arg1));
        },
        __wbg_deleteTexture_4cc3a009d8de3629: function(arg0, arg1) {
            getObject(arg0).deleteTexture(getObject(arg1));
        },
        __wbg_deleteTexture_5f176c5fb10209e7: function(arg0, arg1) {
            getObject(arg0).deleteTexture(getObject(arg1));
        },
        __wbg_deleteVertexArrayOES_8ddbff0c3c4bd01b: function(arg0, arg1) {
            getObject(arg0).deleteVertexArrayOES(getObject(arg1));
        },
        __wbg_deleteVertexArray_a810728c9f325e4c: function(arg0, arg1) {
            getObject(arg0).deleteVertexArray(getObject(arg1));
        },
        __wbg_deltaMode_1f7b0a593ff1ea16: function(arg0) {
            const ret = getObject(arg0).deltaMode;
            return ret;
        },
        __wbg_deltaX_948c3a4bd666561a: function(arg0) {
            const ret = getObject(arg0).deltaX;
            return ret;
        },
        __wbg_deltaY_635e57899c7ee3a1: function(arg0) {
            const ret = getObject(arg0).deltaY;
            return ret;
        },
        __wbg_depthFunc_e3cb63d0ea760f4f: function(arg0, arg1) {
            getObject(arg0).depthFunc(arg1 >>> 0);
        },
        __wbg_depthFunc_fc1a3d2fde41ec92: function(arg0, arg1) {
            getObject(arg0).depthFunc(arg1 >>> 0);
        },
        __wbg_depthMask_77fed42fc2117471: function(arg0, arg1) {
            getObject(arg0).depthMask(arg1 !== 0);
        },
        __wbg_depthMask_e85b064c1ee5147d: function(arg0, arg1) {
            getObject(arg0).depthMask(arg1 !== 0);
        },
        __wbg_depthRange_2b6920405cfb1e3d: function(arg0, arg1, arg2) {
            getObject(arg0).depthRange(arg1, arg2);
        },
        __wbg_depthRange_6e341dd878a6597d: function(arg0, arg1, arg2) {
            getObject(arg0).depthRange(arg1, arg2);
        },
        __wbg_description_a6d69e2e949ce306: function(arg0, arg1) {
            const ret = getObject(arg1).description;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_destroy_ac003021c8b3c54b: function(arg0) {
            getObject(arg0).destroy();
        },
        __wbg_devicePixelContentBoxSize_b18683fceddaada3: function(arg0) {
            const ret = getObject(arg0).devicePixelContentBoxSize;
            return addHeapObject(ret);
        },
        __wbg_devicePixelRatio_da557bfb3b99fb02: function(arg0) {
            const ret = getObject(arg0).devicePixelRatio;
            return ret;
        },
        __wbg_disableVertexAttribArray_45e2fd394cde8686: function(arg0, arg1) {
            getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_disableVertexAttribArray_b985896436c82562: function(arg0, arg1) {
            getObject(arg0).disableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_disable_6907c683adc38e04: function(arg0, arg1) {
            getObject(arg0).disable(arg1 >>> 0);
        },
        __wbg_disable_71ea0d3d0529f100: function(arg0, arg1) {
            getObject(arg0).disable(arg1 >>> 0);
        },
        __wbg_disconnect_cef5c809f09463bb: function(arg0) {
            getObject(arg0).disconnect();
        },
        __wbg_documentElement_02547989338a4499: function(arg0) {
            const ret = getObject(arg0).documentElement;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_document_c7f486c52d63d24e: function(arg0) {
            const ret = getObject(arg0).document;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_downloadText_b0f3f1c330ea0fdd: function(arg0, arg1, arg2, arg3) {
            window.__bookwormBridge.downloadText(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
        },
        __wbg_drawArraysInstancedANGLE_1161faf698564d6d: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).drawArraysInstancedANGLE(arg1 >>> 0, arg2, arg3, arg4);
        },
        __wbg_drawArraysInstanced_0ef1fbcdba111c38: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).drawArraysInstanced(arg1 >>> 0, arg2, arg3, arg4);
        },
        __wbg_drawArrays_983f352c9e94d800: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
        },
        __wbg_drawArrays_bc085b67da45ec03: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).drawArrays(arg1 >>> 0, arg2, arg3);
        },
        __wbg_drawBuffersWEBGL_6cfb1b5e064e9ed2: function(arg0, arg1) {
            getObject(arg0).drawBuffersWEBGL(getObject(arg1));
        },
        __wbg_drawBuffers_9b22ea9d8f6a15f0: function(arg0, arg1) {
            getObject(arg0).drawBuffers(getObject(arg1));
        },
        __wbg_drawElementsInstancedANGLE_da080f7c11a85ead: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).drawElementsInstancedANGLE(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        },
        __wbg_drawElementsInstanced_ea7d6b71beea79a4: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).drawElementsInstanced(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        },
        __wbg_draw_977efcdcd4b12838: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).draw(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_enableVertexAttribArray_ad4d8ff487ea8fe1: function(arg0, arg1) {
            getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_enableVertexAttribArray_db5eef27f6b32c4a: function(arg0, arg1) {
            getObject(arg0).enableVertexAttribArray(arg1 >>> 0);
        },
        __wbg_enable_9fb8e4de75a17155: function(arg0, arg1) {
            getObject(arg0).enable(arg1 >>> 0);
        },
        __wbg_enable_9fca6962aca10f48: function(arg0, arg1) {
            getObject(arg0).enable(arg1 >>> 0);
        },
        __wbg_endQuery_4eea0267f103b625: function(arg0, arg1) {
            getObject(arg0).endQuery(arg1 >>> 0);
        },
        __wbg_end_3bbedaa037606029: function(arg0) {
            getObject(arg0).end();
        },
        __wbg_enqueue_6cb545d22db14f33: function() { return handleError(function (arg0, arg1) {
            getObject(arg0).enqueue(getObject(arg1));
        }, arguments); },
        __wbg_error_200f31277a98b99f: function(arg0) {
            const ret = getObject(arg0).error;
            return addHeapObject(ret);
        },
        __wbg_error_756c5934221e6fee: function(arg0) {
            console.error(getObject(arg0));
        },
        __wbg_error_757e9472f8410341: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_export4(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_eval_7759a61726fa9c38: function() { return handleError(function (arg0, arg1) {
            const ret = eval(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_exitFullscreen_68433121607a460e: function(arg0) {
            getObject(arg0).exitFullscreen();
        },
        __wbg_features_05f2c29f778eefeb: function(arg0) {
            const ret = getObject(arg0).features;
            return addHeapObject(ret);
        },
        __wbg_features_c00dd668abf7e418: function(arg0) {
            const ret = getObject(arg0).features;
            return addHeapObject(ret);
        },
        __wbg_fenceSync_13999b65854e9b9b: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).fenceSync(arg1 >>> 0, arg2 >>> 0);
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_fetch_abfd5c25461aa51d: function() { return handleError(function (arg0) {
            const ret = fetch(getObject(arg0));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_fillText_1f002b9b7fd1f2ac: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).fillText(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_finish_0ce019eb398085de: function(arg0) {
            getObject(arg0).finish();
        },
        __wbg_finish_49529a046631ad9a: function(arg0) {
            const ret = getObject(arg0).finish();
            return addHeapObject(ret);
        },
        __wbg_finish_646a6b6ddc91f563: function(arg0) {
            getObject(arg0).finish();
        },
        __wbg_finish_678df663961386ff: function(arg0, arg1) {
            const ret = getObject(arg0).finish(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_flush_824eea12383b8cca: function(arg0) {
            getObject(arg0).flush();
        },
        __wbg_flush_c4a6f752e79bfc53: function(arg0) {
            getObject(arg0).flush();
        },
        __wbg_focus_79e6ed3164b6185f: function() { return handleError(function (arg0, arg1) {
            getObject(arg0).focus(getObject(arg1));
        }, arguments); },
        __wbg_framebufferRenderbuffer_aa5275e6a52dc6bd: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4));
        },
        __wbg_framebufferRenderbuffer_f5e60406b8ee912e: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).framebufferRenderbuffer(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4));
        },
        __wbg_framebufferTexture2D_7a85448ff0ab4b2c: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4), arg5);
        },
        __wbg_framebufferTexture2D_ee6a0d85fd7083aa: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).framebufferTexture2D(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, getObject(arg4), arg5);
        },
        __wbg_framebufferTextureLayer_7c877ec2a0f9220b: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).framebufferTextureLayer(arg1 >>> 0, arg2 >>> 0, getObject(arg3), arg4, arg5);
        },
        __wbg_framebufferTextureMultiviewOVR_64b070ac25c07833: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).framebufferTextureMultiviewOVR(arg1 >>> 0, arg2 >>> 0, getObject(arg3), arg4, arg5, arg6);
        },
        __wbg_from_a39669ce566077da: function(arg0) {
            const ret = Array.from(getObject(arg0));
            return addHeapObject(ret);
        },
        __wbg_frontFace_77f09a7664c8cd54: function(arg0, arg1) {
            getObject(arg0).frontFace(arg1 >>> 0);
        },
        __wbg_frontFace_b3888c65efdb32fe: function(arg0, arg1) {
            getObject(arg0).frontFace(arg1 >>> 0);
        },
        __wbg_fullscreenElement_ed7ec71fb58bcc4f: function(arg0) {
            const ret = getObject(arg0).fullscreenElement;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_getAsFile_e64a2869e682dcc6: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).getAsFile();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_getAttribute_b12f8535a2cb5368: function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg1).getAttribute(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getBoundingClientRect_299f40f8641457e6: function(arg0) {
            const ret = getObject(arg0).getBoundingClientRect();
            return addHeapObject(ret);
        },
        __wbg_getBufferSubData_2c901ce3bd14301e: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).getBufferSubData(arg1 >>> 0, arg2, getObject(arg3));
        },
        __wbg_getContext_2c75bba1a1c9739b: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2), getObject(arg3));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_getContext_5ff6bd600503b094: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_getContext_db25e9527f2ccc08: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2), getObject(arg3));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_getContext_e0c05ffee530bdcf: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_getCurrentTexture_de7133bb1a828045: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).getCurrentTexture();
            return addHeapObject(ret);
        }, arguments); },
        __wbg_getData_b19c87e3887bc0cb: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = getObject(arg1).getData(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_getExtension_112db5d69fdfc10f: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).getExtension(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_getImageData_6c41252bbdd1a813: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = getObject(arg0).getImageData(arg1, arg2, arg3, arg4);
            return addHeapObject(ret);
        }, arguments); },
        __wbg_getIndexedParameter_d4437eb4ad4a0bfa: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).getIndexedParameter(arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        }, arguments); },
        __wbg_getModifierState_eacebc4e0503bcfc: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getModifierState(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_getOwnPropertyDescriptor_4a4c6c46eaaf8da4: function(arg0, arg1) {
            const ret = Object.getOwnPropertyDescriptor(getObject(arg0), getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_getParameter_031885c3618c2d0c: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).getParameter(arg1 >>> 0);
            return addHeapObject(ret);
        }, arguments); },
        __wbg_getParameter_7f269432cd46eb73: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).getParameter(arg1 >>> 0);
            return addHeapObject(ret);
        }, arguments); },
        __wbg_getPreferredCanvasFormat_f775f4ebe1768892: function(arg0) {
            const ret = getObject(arg0).getPreferredCanvasFormat();
            return (__wbindgen_enum_GpuTextureFormat.indexOf(ret) + 1 || 102) - 1;
        },
        __wbg_getProgramInfoLog_0e335b65673afba8: function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getProgramInfoLog_82aa11c9d7e8d96e: function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getProgramInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getProgramParameter_1a93fd2f4dcc0622: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_getProgramParameter_2a5860ab824690ec: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getProgramParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_getQueryParameter_145128cfab5de56d: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getQueryParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_getRandomValues_436a51d0629d84e1: function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_getRandomValues_c5a70fc1fff5b3c7: function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_getReader_38a152de4ceaae00: function(arg0) {
            const ret = getObject(arg0).getReader();
            return addHeapObject(ret);
        },
        __wbg_getShaderInfoLog_416aea280f6fdf84: function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getShaderInfoLog_e66ab6619bcd208d: function(arg0, arg1, arg2) {
            const ret = getObject(arg1).getShaderInfoLog(getObject(arg2));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getShaderParameter_b19fb6adcff77066: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_getShaderParameter_d542b1412563ae42: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getShaderParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_getSupportedExtensions_67cde11dd49cad1a: function(arg0) {
            const ret = getObject(arg0).getSupportedExtensions();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_getSupportedProfiles_abd53cd63178e1e0: function(arg0) {
            const ret = getObject(arg0).getSupportedProfiles();
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_getSyncParameter_f4eeca8570f06af5: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getSyncParameter(getObject(arg1), arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_getTime_65922ba0b59d55a7: function(arg0) {
            const ret = getObject(arg0).getTime();
            return ret;
        },
        __wbg_getTimezoneOffset_6e4850ad528ac37d: function(arg0) {
            const ret = getObject(arg0).getTimezoneOffset();
            return ret;
        },
        __wbg_getType_c50c63e33d04df50: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).getType(getStringFromWasm0(arg1, arg2));
            return addHeapObject(ret);
        },
        __wbg_getUniformBlockIndex_db57fe2d6ed6af2a: function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getUniformBlockIndex(getObject(arg1), getStringFromWasm0(arg2, arg3));
            return ret;
        },
        __wbg_getUniformLocation_03b9677a8f868582: function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_getUniformLocation_20bad5ce4a0aaae4: function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).getUniformLocation(getObject(arg1), getStringFromWasm0(arg2, arg3));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_get_501ad1d6b5ba20fe: function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_get_95e4d462165c92ae: function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_get_989d0a1309644f2b: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(getObject(arg0), getObject(arg1));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_get_b1f0ab13c737f856: function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return addHeapObject(ret);
        },
        __wbg_get_done_a668aa62d81fad70: function(arg0) {
            const ret = getObject(arg0).done;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg_get_unchecked_363572bdd397d473: function(arg0, arg1) {
            const ret = getObject(arg0)[arg1 >>> 0];
            return addHeapObject(ret);
        },
        __wbg_get_value_6c62a77c168d825a: function(arg0) {
            const ret = getObject(arg0).value;
            return addHeapObject(ret);
        },
        __wbg_gpu_c7d57fb68edbac19: function(arg0) {
            const ret = getObject(arg0).gpu;
            return addHeapObject(ret);
        },
        __wbg_hardwareConcurrency_8d59b9ac64b051ee: function(arg0) {
            const ret = getObject(arg0).hardwareConcurrency;
            return ret;
        },
        __wbg_hasFocus_e538c8e02f94e4de: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).hasFocus();
            return ret;
        }, arguments); },
        __wbg_has_464f8b9416279d65: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.has(getObject(arg0), getObject(arg1));
            return ret;
        }, arguments); },
        __wbg_has_47b8eb16db92789a: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).has(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_headers_2594464ff62969f3: function(arg0) {
            const ret = getObject(arg0).headers;
            return addHeapObject(ret);
        },
        __wbg_headers_a1e9854406915ee7: function(arg0) {
            const ret = getObject(arg0).headers;
            return addHeapObject(ret);
        },
        __wbg_height_08b0827eb99f89df: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).height;
            return ret;
        }, arguments); },
        __wbg_height_24586a23e4789d28: function(arg0) {
            const ret = getObject(arg0).height;
            return ret;
        },
        __wbg_height_448f88704965f1f7: function(arg0) {
            const ret = getObject(arg0).height;
            return ret;
        },
        __wbg_height_c15ee46a1345e9af: function(arg0) {
            const ret = getObject(arg0).height;
            return ret;
        },
        __wbg_height_d011730144e088ea: function(arg0) {
            const ret = getObject(arg0).height;
            return ret;
        },
        __wbg_height_f186a04cfefb51d0: function(arg0) {
            const ret = getObject(arg0).height;
            return ret;
        },
        __wbg_includes_c0f578d8314b8dbf: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).includes(getObject(arg1), arg2);
            return ret;
        },
        __wbg_info_3c45b8cfbbacab3b: function(arg0) {
            const ret = getObject(arg0).info;
            return addHeapObject(ret);
        },
        __wbg_info_f25c01bee103395d: function(arg0) {
            console.info(getObject(arg0));
        },
        __wbg_inlineSize_199278605b83587a: function(arg0) {
            const ret = getObject(arg0).inlineSize;
            return ret;
        },
        __wbg_innerHeight_f0945dfd6f2d3a87: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).innerHeight;
            return addHeapObject(ret);
        }, arguments); },
        __wbg_innerWidth_1553ed0783afe5b4: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).innerWidth;
            return addHeapObject(ret);
        }, arguments); },
        __wbg_inputType_a845f09b4eb190ca: function(arg0, arg1) {
            const ret = getObject(arg1).inputType;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_instanceof_DedicatedWorkerGlobalScope_a4848cd879d10143: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof DedicatedWorkerGlobalScope;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_GpuOutOfMemoryError_829d8185d83170f0: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof GPUOutOfMemoryError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_GpuValidationError_83f906582a640f23: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof GPUValidationError;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlCanvasElement_4d7e131643d814c1: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof HTMLCanvasElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlElement_bd89ce6abf9dad44: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof HTMLElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlTextAreaElement_632129eba25e4367: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof HTMLTextAreaElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_OffscreenCanvasRenderingContext2d_4b64493c5055d621: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof OffscreenCanvasRenderingContext2D;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_PointerEvent_4269e92f16379aea: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof PointerEvent;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_ReadableStreamDefaultReader_a847f0465a2b7a2f: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof ReadableStreamDefaultReader;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_ResizeObserverEntry_e1c4a1881300a519: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof ResizeObserverEntry;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Response_6366a785e400039b: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof Response;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_SharedArrayBuffer_9e7bc3c6c8906969: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof SharedArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_598adc0fef426aa8: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_WebGl2RenderingContext_015d221b904a3c7b: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof WebGL2RenderingContext;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_a3b8566f0a9c5d1a: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_WorkerGlobalScope_0c1a6411473b843f: function(arg0) {
            let result;
            try {
                result = getObject(arg0) instanceof WorkerGlobalScope;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_invalidateFramebuffer_4538559f02a556ae: function() { return handleError(function (arg0, arg1, arg2) {
            getObject(arg0).invalidateFramebuffer(arg1 >>> 0, getObject(arg2));
        }, arguments); },
        __wbg_isArray_5674713bb7b79043: function(arg0) {
            const ret = Array.isArray(getObject(arg0));
            return ret;
        },
        __wbg_isComposing_2ab72d608a0a8d0e: function(arg0) {
            const ret = getObject(arg0).isComposing;
            return ret;
        },
        __wbg_isComposing_7d4f39b2d2fa2a3c: function(arg0) {
            const ret = getObject(arg0).isComposing;
            return ret;
        },
        __wbg_isFallbackAdapter_3acf34ef5402fce5: function(arg0) {
            const ret = getObject(arg0).isFallbackAdapter;
            return ret;
        },
        __wbg_is_61443cc073056436: function(arg0, arg1) {
            const ret = Object.is(getObject(arg0), getObject(arg1));
            return ret;
        },
        __wbg_items_a8fdc4cf728ef922: function(arg0) {
            const ret = getObject(arg0).items;
            return addHeapObject(ret);
        },
        __wbg_key_d29d670923b81510: function(arg0, arg1) {
            const ret = getObject(arg1).key;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_kind_ed2088ba17535d91: function(arg0, arg1) {
            const ret = getObject(arg1).kind;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_label_0752a49b0bb6f2c4: function(arg0, arg1) {
            const ret = getObject(arg1).label;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_left_644de747afd08e57: function(arg0) {
            const ret = getObject(arg0).left;
            return ret;
        },
        __wbg_length_31bdaf014f5fbde2: function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        },
        __wbg_length_4e1adc0d42e23620: function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        },
        __wbg_length_b9633f9f50fac3c7: function(arg0) {
            const ret = getObject(arg0).length;
            return ret;
        },
        __wbg_limits_a55115608ac6de8d: function(arg0) {
            const ret = getObject(arg0).limits;
            return addHeapObject(ret);
        },
        __wbg_limits_f4203404018656d1: function(arg0) {
            const ret = getObject(arg0).limits;
            return addHeapObject(ret);
        },
        __wbg_linkProgram_794fd290a759542a: function(arg0, arg1) {
            getObject(arg0).linkProgram(getObject(arg1));
        },
        __wbg_linkProgram_e408ee5f93686f1f: function(arg0, arg1) {
            getObject(arg0).linkProgram(getObject(arg1));
        },
        __wbg_log_363d83b9114c8831: function(arg0) {
            console.log(getObject(arg0));
        },
        __wbg_lost_fa085c0cf1bffae4: function(arg0) {
            const ret = getObject(arg0).lost;
            return addHeapObject(ret);
        },
        __wbg_mapAsync_84aeaa5780c9b8c7: function(arg0, arg1, arg2, arg3) {
            const ret = getObject(arg0).mapAsync(arg1 >>> 0, arg2, arg3);
            return addHeapObject(ret);
        },
        __wbg_matchMedia_0e1d9f8bfb0f1f46: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).matchMedia(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_matches_0c791086704a6cb5: function(arg0) {
            const ret = getObject(arg0).matches;
            return ret;
        },
        __wbg_maxBindGroupsPlusVertexBuffers_d6dffc4ad89f024c: function(arg0) {
            const ret = getObject(arg0).maxBindGroupsPlusVertexBuffers;
            return ret;
        },
        __wbg_maxBindGroups_e8bcce17037d7413: function(arg0) {
            const ret = getObject(arg0).maxBindGroups;
            return ret;
        },
        __wbg_maxBindingsPerBindGroup_ef08905e194a1e54: function(arg0) {
            const ret = getObject(arg0).maxBindingsPerBindGroup;
            return ret;
        },
        __wbg_maxBufferSize_b1db4c8964be17cf: function(arg0) {
            const ret = getObject(arg0).maxBufferSize;
            return ret;
        },
        __wbg_maxColorAttachmentBytesPerSample_ac08ed619898c87e: function(arg0) {
            const ret = getObject(arg0).maxColorAttachmentBytesPerSample;
            return ret;
        },
        __wbg_maxColorAttachments_d1f42a41baed04ad: function(arg0) {
            const ret = getObject(arg0).maxColorAttachments;
            return ret;
        },
        __wbg_maxComputeInvocationsPerWorkgroup_2fee70c64ef20a73: function(arg0) {
            const ret = getObject(arg0).maxComputeInvocationsPerWorkgroup;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeX_2c2884390e0f2002: function(arg0) {
            const ret = getObject(arg0).maxComputeWorkgroupSizeX;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeY_a6dbdf413372949b: function(arg0) {
            const ret = getObject(arg0).maxComputeWorkgroupSizeY;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeZ_40ec8315d6a923ec: function(arg0) {
            const ret = getObject(arg0).maxComputeWorkgroupSizeZ;
            return ret;
        },
        __wbg_maxComputeWorkgroupStorageSize_d0d1d2e746e81bcf: function(arg0) {
            const ret = getObject(arg0).maxComputeWorkgroupStorageSize;
            return ret;
        },
        __wbg_maxComputeWorkgroupsPerDimension_61b16abcf0d80b79: function(arg0) {
            const ret = getObject(arg0).maxComputeWorkgroupsPerDimension;
            return ret;
        },
        __wbg_maxDynamicStorageBuffersPerPipelineLayout_797d17e6e6944fdb: function(arg0) {
            const ret = getObject(arg0).maxDynamicStorageBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxDynamicUniformBuffersPerPipelineLayout_edb82f86942b7949: function(arg0) {
            const ret = getObject(arg0).maxDynamicUniformBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxInterStageShaderVariables_908ab3a923f07870: function(arg0) {
            const ret = getObject(arg0).maxInterStageShaderVariables;
            return ret;
        },
        __wbg_maxSampledTexturesPerShaderStage_191dbad7e2506090: function(arg0) {
            const ret = getObject(arg0).maxSampledTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxSamplersPerShaderStage_cf1a5d29de6387c1: function(arg0) {
            const ret = getObject(arg0).maxSamplersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageBufferBindingSize_6c66dbb2e10b3ebf: function(arg0) {
            const ret = getObject(arg0).maxStorageBufferBindingSize;
            return ret;
        },
        __wbg_maxStorageBuffersPerShaderStage_e9cde31564b6860e: function(arg0) {
            const ret = getObject(arg0).maxStorageBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageTexturesPerShaderStage_80c81f127106f576: function(arg0) {
            const ret = getObject(arg0).maxStorageTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxTextureArrayLayers_2315e52a2a6ad4f6: function(arg0) {
            const ret = getObject(arg0).maxTextureArrayLayers;
            return ret;
        },
        __wbg_maxTextureDimension1D_8adabbb69e668b5d: function(arg0) {
            const ret = getObject(arg0).maxTextureDimension1D;
            return ret;
        },
        __wbg_maxTextureDimension2D_9023132dcda08d76: function(arg0) {
            const ret = getObject(arg0).maxTextureDimension2D;
            return ret;
        },
        __wbg_maxTextureDimension3D_bb2e13eec8c6a78b: function(arg0) {
            const ret = getObject(arg0).maxTextureDimension3D;
            return ret;
        },
        __wbg_maxTouchPoints_aa4db34bc3dbf1fb: function(arg0) {
            const ret = getObject(arg0).maxTouchPoints;
            return ret;
        },
        __wbg_maxUniformBufferBindingSize_825882c54699eb99: function(arg0) {
            const ret = getObject(arg0).maxUniformBufferBindingSize;
            return ret;
        },
        __wbg_maxUniformBuffersPerShaderStage_5b1f83edc54e5013: function(arg0) {
            const ret = getObject(arg0).maxUniformBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxVertexAttributes_a026b2a337f0bdd5: function(arg0) {
            const ret = getObject(arg0).maxVertexAttributes;
            return ret;
        },
        __wbg_maxVertexBufferArrayStride_ba288c75efda40ac: function(arg0) {
            const ret = getObject(arg0).maxVertexBufferArrayStride;
            return ret;
        },
        __wbg_maxVertexBuffers_b40d07717d2856fd: function(arg0) {
            const ret = getObject(arg0).maxVertexBuffers;
            return ret;
        },
        __wbg_measureText_7e962c056fc1a5e9: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).measureText(getStringFromWasm0(arg1, arg2));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_message_31dd74b6690b5316: function(arg0, arg1) {
            const ret = getObject(arg1).message;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_message_6f556f23bc8a4aff: function(arg0, arg1) {
            const ret = getObject(arg1).message;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_metaKey_3f9183dfd5e1b726: function(arg0) {
            const ret = getObject(arg0).metaKey;
            return ret;
        },
        __wbg_metaKey_feb71e77f26b1a4c: function(arg0) {
            const ret = getObject(arg0).metaKey;
            return ret;
        },
        __wbg_minStorageBufferOffsetAlignment_2c84ffed8659a936: function(arg0) {
            const ret = getObject(arg0).minStorageBufferOffsetAlignment;
            return ret;
        },
        __wbg_minUniformBufferOffsetAlignment_35a3aa8f239bba4a: function(arg0) {
            const ret = getObject(arg0).minUniformBufferOffsetAlignment;
            return ret;
        },
        __wbg_navigator_d217ca64c4bbff48: function(arg0) {
            const ret = getObject(arg0).navigator;
            return addHeapObject(ret);
        },
        __wbg_navigator_d25c0f071226f233: function(arg0) {
            const ret = getObject(arg0).navigator;
            return addHeapObject(ret);
        },
        __wbg_new_0_35540e542ba689d2: function() {
            const ret = new Date();
            return addHeapObject(ret);
        },
        __wbg_new_12e5d807044fbfe3: function() { return handleError(function (arg0, arg1) {
            const ret = new OffscreenCanvas(arg0 >>> 0, arg1 >>> 0);
            return addHeapObject(ret);
        }, arguments); },
        __wbg_new_180f1022bb6ee517: function(arg0) {
            const ret = new Date(getObject(arg0));
            return addHeapObject(ret);
        },
        __wbg_new_1da3429bc3c4541c: function(arg0) {
            const ret = new Uint8Array(getObject(arg0));
            return addHeapObject(ret);
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return addHeapObject(ret);
        },
        __wbg_new_a32a1ab6c6655abe: function(arg0, arg1) {
            const ret = new Error(getStringFromWasm0(arg0, arg1));
            return addHeapObject(ret);
        },
        __wbg_new_bebc3f4757acf305: function() {
            const ret = new Object();
            return addHeapObject(ret);
        },
        __wbg_new_d0cecdd4b12bd7a1: function() { return handleError(function (arg0) {
            const ret = new ResizeObserver(getObject(arg0));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_new_ffa92086ea89f79c: function() {
            const ret = new Array();
            return addHeapObject(ret);
        },
        __wbg_new_from_slice_4ee02165f9de919e: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return addHeapObject(ret);
        },
        __wbg_new_typed_6f8b0d724fe26c07: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return __wasm_bindgen_func_elem_26683(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return addHeapObject(ret);
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_typed_7d4574ab4b8446c8: function() {
            const ret = new Object();
            return addHeapObject(ret);
        },
        __wbg_new_with_byte_offset_and_length_492c969e8b5da8a4: function(arg0, arg1, arg2) {
            const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_new_with_byte_offset_and_length_edceafd3cbaf9910: function(arg0, arg1, arg2) {
            const ret = new Int32Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
            return addHeapObject(ret);
        },
        __wbg_new_with_options_695bfde16ea544ed: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new Worker(getStringFromWasm0(arg0, arg1), getObject(arg2));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_new_with_str_and_init_2f31a77deda127fd: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new Request(getStringFromWasm0(arg0, arg1), getObject(arg2));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_new_with_str_sequence_fc628905fc56e640: function() { return handleError(function (arg0) {
            const ret = new Blob(getObject(arg0));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_notify_00ac52e7dfc01c10: function() { return handleError(function (arg0, arg1) {
            const ret = Atomics.notify(getObject(arg0), arg1 >>> 0);
            return ret;
        }, arguments); },
        __wbg_now_0b7736bc17fd7719: function(arg0) {
            const ret = getObject(arg0).now();
            return ret;
        },
        __wbg_now_d1fb6650485d7f3e: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_now_e7c6795a7f81e10f: function(arg0) {
            const ret = getObject(arg0).now();
            return ret;
        },
        __wbg_observe_232627fdd95d3d31: function(arg0, arg1) {
            getObject(arg0).observe(getObject(arg1));
        },
        __wbg_observe_756a715b58148858: function(arg0, arg1, arg2) {
            getObject(arg0).observe(getObject(arg1), getObject(arg2));
        },
        __wbg_of_1a3f47170341cefe: function(arg0) {
            const ret = Array.of(getObject(arg0));
            return addHeapObject(ret);
        },
        __wbg_offsetLeft_d9aed195ef935283: function(arg0) {
            const ret = getObject(arg0).offsetLeft;
            return ret;
        },
        __wbg_offsetTop_9ca390a8db540897: function(arg0) {
            const ret = getObject(arg0).offsetTop;
            return ret;
        },
        __wbg_offsetX_5b637acbf4fc40e8: function(arg0) {
            const ret = getObject(arg0).offsetX;
            return ret;
        },
        __wbg_offsetY_ad9ca8949500ce96: function(arg0) {
            const ret = getObject(arg0).offsetY;
            return ret;
        },
        __wbg_onError_46f378082b854779: function(arg0, arg1) {
            window.__bookwormBridge.onError(getStringFromWasm0(arg0, arg1));
        },
        __wbg_onReady_162c6e7384d1dbd2: function() {
            window.__bookwormBridge.onReady();
        },
        __wbg_onSubmittedWorkDone_f62945abb48d8a84: function(arg0) {
            const ret = getObject(arg0).onSubmittedWorkDone();
            return addHeapObject(ret);
        },
        __wbg_open_ca59cebf365379c9: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).open(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_performance_3fcf6e32a7e1ed0a: function(arg0) {
            const ret = getObject(arg0).performance;
            return addHeapObject(ret);
        },
        __wbg_pickEpubFiles_7589952e18ad4f29: function() {
            window.__bookwormBridge.pickEpubFiles();
        },
        __wbg_pixelStorei_1bc7980b82e1ba4b: function(arg0, arg1, arg2) {
            getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
        },
        __wbg_pixelStorei_d540c25aafa93789: function(arg0, arg1, arg2) {
            getObject(arg0).pixelStorei(arg1 >>> 0, arg2);
        },
        __wbg_platform_61cb3024b6a10dd0: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).platform;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_pointerId_b8c496f0172b1473: function(arg0) {
            const ret = getObject(arg0).pointerId;
            return ret;
        },
        __wbg_pointerType_753a9c4f0e3018cc: function(arg0, arg1) {
            const ret = getObject(arg1).pointerType;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_polygonOffset_a97069c9fe385bff: function(arg0, arg1, arg2) {
            getObject(arg0).polygonOffset(arg1, arg2);
        },
        __wbg_polygonOffset_f99887a0cc3d7df4: function(arg0, arg1, arg2) {
            getObject(arg0).polygonOffset(arg1, arg2);
        },
        __wbg_postMessage_14c07434f6b7e150: function() { return handleError(function (arg0, arg1) {
            getObject(arg0).postMessage(getObject(arg1));
        }, arguments); },
        __wbg_postMessage_1f51e50254674143: function() { return handleError(function (arg0, arg1) {
            getObject(arg0).postMessage(getObject(arg1));
        }, arguments); },
        __wbg_preventDefault_4235a4ccf8540533: function(arg0) {
            getObject(arg0).preventDefault();
        },
        __wbg_prototypesetcall_ae9f5e7459250748: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), getObject(arg2));
        },
        __wbg_push_bfdf956ba476f65b: function(arg0, arg1) {
            const ret = getObject(arg0).push(getObject(arg1));
            return ret;
        },
        __wbg_queryCounterEXT_c9fb8ad39c35fdf6: function(arg0, arg1, arg2) {
            getObject(arg0).queryCounterEXT(getObject(arg1), arg2 >>> 0);
        },
        __wbg_querySelectorAll_fdbf93e11103921d: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).querySelectorAll(getStringFromWasm0(arg1, arg2));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_querySelector_5b1d9ebad373dc9d: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).querySelector(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        }, arguments); },
        __wbg_queueMicrotask_85c90f6987555d65: function(arg0) {
            const ret = getObject(arg0).queueMicrotask;
            return addHeapObject(ret);
        },
        __wbg_queueMicrotask_f173251ad502837f: function(arg0, arg1) {
            getObject(arg0).queueMicrotask(getObject(arg1));
        },
        __wbg_queueMicrotask_f6a1fa10b81d1fc0: function(arg0) {
            queueMicrotask(getObject(arg0));
        },
        __wbg_queue_6e272de986f8f6fc: function(arg0) {
            const ret = getObject(arg0).queue;
            return addHeapObject(ret);
        },
        __wbg_random_7bce4e9e78e2e0d0: function() {
            const ret = Math.random();
            return ret;
        },
        __wbg_readBuffer_1679a4b6dcd78e30: function(arg0, arg1) {
            getObject(arg0).readBuffer(arg1 >>> 0);
        },
        __wbg_readPixels_5d75a0ec1e2aab33: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, getObject(arg7));
        }, arguments); },
        __wbg_readPixels_c60bfe6126f65619: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, arg7);
        }, arguments); },
        __wbg_readPixels_dcde619ca94b6296: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            getObject(arg0).readPixels(arg1, arg2, arg3, arg4, arg5 >>> 0, arg6 >>> 0, getObject(arg7));
        }, arguments); },
        __wbg_read_2f356b314071c943: function(arg0) {
            const ret = getObject(arg0).read();
            return addHeapObject(ret);
        },
        __wbg_read_31091533ffadf971: function(arg0) {
            const ret = getObject(arg0).read();
            return addHeapObject(ret);
        },
        __wbg_reason_33b7423157d5e170: function(arg0) {
            const ret = getObject(arg0).reason;
            return (__wbindgen_enum_GpuDeviceLostReason.indexOf(ret) + 1 || 3) - 1;
        },
        __wbg_removeAttribute_8ce0ab2fa42b383d: function() { return handleError(function (arg0, arg1, arg2) {
            getObject(arg0).removeAttribute(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_removeEventListener_4c9706814dcfaf79: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
        }, arguments); },
        __wbg_remove_2d9ed324f8c6126a: function(arg0) {
            getObject(arg0).remove();
        },
        __wbg_renderbufferStorageMultisample_5d9012ce50f133e3: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).renderbufferStorageMultisample(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        },
        __wbg_renderbufferStorage_2b50da85624a8b73: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
        },
        __wbg_renderbufferStorage_2eb3d4343d1a1c6e: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).renderbufferStorage(arg1 >>> 0, arg2 >>> 0, arg3, arg4);
        },
        __wbg_repeat_01e85ccb1bcef326: function(arg0) {
            const ret = getObject(arg0).repeat;
            return ret;
        },
        __wbg_requestAdapter_2cb1006baf7abd60: function(arg0) {
            const ret = getObject(arg0).requestAdapter();
            return addHeapObject(ret);
        },
        __wbg_requestAdapter_b725cba92e32c4ce: function(arg0, arg1) {
            const ret = getObject(arg0).requestAdapter(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_requestAnimationFrame_4988887658fcee2e: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
            return ret;
        }, arguments); },
        __wbg_requestDevice_45bc1149c3b98eaf: function(arg0, arg1) {
            const ret = getObject(arg0).requestDevice(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_requestFullscreen_00cf0a222b2101eb: function() { return handleError(function (arg0) {
            getObject(arg0).requestFullscreen();
        }, arguments); },
        __wbg_requestIdleCallback_36e811d17034de10: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).requestIdleCallback(getObject(arg1), getObject(arg2));
            return ret;
        }, arguments); },
        __wbg_requestIdleCallback_a3ab6672665bc9a2: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).requestIdleCallback(getObject(arg1));
            return ret;
        }, arguments); },
        __wbg_resolve_35ec7e0c6af4c82c: function(arg0) {
            const ret = Promise.resolve(getObject(arg0));
            return addHeapObject(ret);
        },
        __wbg_respond_83a71686e927ca32: function() { return handleError(function (arg0, arg1) {
            getObject(arg0).respond(arg1 >>> 0);
        }, arguments); },
        __wbg_right_3b45e4b285b3f490: function(arg0) {
            const ret = getObject(arg0).right;
            return ret;
        },
        __wbg_samplerParameterf_34605394359af7ae: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).samplerParameterf(getObject(arg1), arg2 >>> 0, arg3);
        },
        __wbg_samplerParameteri_ff1f558c8f43a220: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).samplerParameteri(getObject(arg1), arg2 >>> 0, arg3);
        },
        __wbg_saveBook_c8a56697fc8155b9: function(arg0, arg1, arg2, arg3) {
            window.__bookwormBridge.saveBook(getStringFromWasm0(arg0, arg1), getArrayU8FromWasm0(arg2, arg3));
        },
        __wbg_saveConfigJson_08abcbee00cc8fdf: function(arg0, arg1) {
            window.__bookwormBridge.saveConfigJson(getStringFromWasm0(arg0, arg1));
        },
        __wbg_saveLibraryJson_d8e878a00cc3ce6f: function(arg0, arg1) {
            window.__bookwormBridge.saveLibraryJson(getStringFromWasm0(arg0, arg1));
        },
        __wbg_scale_b78c4054a045c697: function(arg0) {
            const ret = getObject(arg0).scale;
            return ret;
        },
        __wbg_scissor_c53448a6fa939bfb: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).scissor(arg1, arg2, arg3, arg4);
        },
        __wbg_scissor_d896348c98d0c9ac: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).scissor(arg1, arg2, arg3, arg4);
        },
        __wbg_screen_1bdd74acb98e2b0d: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).screen;
            return addHeapObject(ret);
        }, arguments); },
        __wbg_selectionEnd_f22fffc2d6a8e2fc: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).selectionEnd;
            return isLikeNone(ret) ? Number.MAX_SAFE_INTEGER : (ret) >>> 0;
        }, arguments); },
        __wbg_selectionStart_41fde58c23be3c21: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).selectionStart;
            return isLikeNone(ret) ? Number.MAX_SAFE_INTEGER : (ret) >>> 0;
        }, arguments); },
        __wbg_setAttribute_8c84e9351986b1f6: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setBindGroup_c3a39fd8d0149600: function(arg0, arg1, arg2) {
            getObject(arg0).setBindGroup(arg1 >>> 0, getObject(arg2));
        },
        __wbg_setBindGroup_e94a2cd341685dbc: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).setBindGroup(arg1 >>> 0, getObject(arg2), getArrayU32FromWasm0(arg3, arg4), arg5, arg6 >>> 0);
        }, arguments); },
        __wbg_setPipeline_823fe7d48236b840: function(arg0, arg1) {
            getObject(arg0).setPipeline(getObject(arg1));
        },
        __wbg_setPointerCapture_b8446eaac3edc11e: function() { return handleError(function (arg0, arg1) {
            getObject(arg0).setPointerCapture(arg1);
        }, arguments); },
        __wbg_setProperty_6139dd07828b5817: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setSelectionRange_f95d233357782d19: function() { return handleError(function (arg0, arg1, arg2) {
            getObject(arg0).setSelectionRange(arg1 >>> 0, arg2 >>> 0);
        }, arguments); },
        __wbg_setTimeout_7b1dfbeaccbf9bb6: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
            return ret;
        }, arguments); },
        __wbg_setTimeout_f511da46f7279b2b: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg0).setTimeout(getObject(arg1));
            return ret;
        }, arguments); },
        __wbg_set_13d25b81ab403f5e: function(arg0, arg1, arg2) {
            getObject(arg0)[arg1 >>> 0] = takeObject(arg2);
        },
        __wbg_set_5f2ad37e5e02dc7b: function(arg0, arg1, arg2) {
            getObject(arg0).set(getArrayU8FromWasm0(arg1, arg2));
        },
        __wbg_set_7923e5ea63b41e6b: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).set(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_set_a377297433dfea63: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
            return ret;
        }, arguments); },
        __wbg_set_a_1fdf746cc2b8222e: function(arg0, arg1) {
            getObject(arg0).a = arg1;
        },
        __wbg_set_access_d85c61c63a8f0054: function(arg0, arg1) {
            getObject(arg0).access = __wbindgen_enum_GpuStorageTextureAccess[arg1];
        },
        __wbg_set_address_mode_u_422d09a7c7c6808b: function(arg0, arg1) {
            getObject(arg0).addressModeU = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_address_mode_v_995b8eaecea12eb2: function(arg0, arg1) {
            getObject(arg0).addressModeV = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_address_mode_w_3d44c8d95e17713b: function(arg0, arg1) {
            getObject(arg0).addressModeW = __wbindgen_enum_GpuAddressMode[arg1];
        },
        __wbg_set_alpha_9f269bbd6ece367d: function(arg0, arg1) {
            getObject(arg0).alpha = getObject(arg1);
        },
        __wbg_set_alpha_mode_64fac09c2c8ce2a3: function(arg0, arg1) {
            getObject(arg0).alphaMode = __wbindgen_enum_GpuCanvasAlphaMode[arg1];
        },
        __wbg_set_alpha_to_coverage_enabled_325bc46f0e28d1e1: function(arg0, arg1) {
            getObject(arg0).alphaToCoverageEnabled = arg1 !== 0;
        },
        __wbg_set_array_layer_count_bffa36fc24b79df0: function(arg0, arg1) {
            getObject(arg0).arrayLayerCount = arg1 >>> 0;
        },
        __wbg_set_array_stride_f64_6aa9b1b64c512846: function(arg0, arg1) {
            getObject(arg0).arrayStride = arg1;
        },
        __wbg_set_aspect_27f3eed305bd8169: function(arg0, arg1) {
            getObject(arg0).aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        },
        __wbg_set_aspect_f5a98aa400a5a3a5: function(arg0, arg1) {
            getObject(arg0).aspect = __wbindgen_enum_GpuTextureAspect[arg1];
        },
        __wbg_set_attributes_bf0350bfb0e148d9: function(arg0, arg1, arg2) {
            getObject(arg0).attributes = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_b_f3fe26f326729156: function(arg0, arg1) {
            getObject(arg0).b = arg1;
        },
        __wbg_set_base_array_layer_343a636d6599095b: function(arg0, arg1) {
            getObject(arg0).baseArrayLayer = arg1 >>> 0;
        },
        __wbg_set_base_mip_level_15a401680a308b29: function(arg0, arg1) {
            getObject(arg0).baseMipLevel = arg1 >>> 0;
        },
        __wbg_set_beginning_of_pass_write_index_be1b8c649beccb0d: function(arg0, arg1) {
            getObject(arg0).beginningOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_bind_group_layouts_3586ff59b180c6f6: function(arg0, arg1, arg2) {
            getObject(arg0).bindGroupLayouts = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_binding_5c2fb74e9df71acc: function(arg0, arg1) {
            getObject(arg0).binding = arg1 >>> 0;
        },
        __wbg_set_binding_dd48e769b9abd49c: function(arg0, arg1) {
            getObject(arg0).binding = arg1 >>> 0;
        },
        __wbg_set_blend_06a2cc3d12d05417: function(arg0, arg1) {
            getObject(arg0).blend = getObject(arg1);
        },
        __wbg_set_body_f39cee72c74a5b02: function(arg0, arg1) {
            getObject(arg0).body = getObject(arg1);
        },
        __wbg_set_box_334f2086d2b008d0: function(arg0, arg1) {
            getObject(arg0).box = __wbindgen_enum_ResizeObserverBoxOptions[arg1];
        },
        __wbg_set_buffer_3446218584f17583: function(arg0, arg1) {
            getObject(arg0).buffer = getObject(arg1);
        },
        __wbg_set_buffer_9924a87f5165fce6: function(arg0, arg1) {
            getObject(arg0).buffer = getObject(arg1);
        },
        __wbg_set_buffers_b16f472e92a88793: function(arg0, arg1, arg2) {
            getObject(arg0).buffers = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_bytes_per_row_28510a23c97e58c2: function(arg0, arg1) {
            getObject(arg0).bytesPerRow = arg1 >>> 0;
        },
        __wbg_set_clear_value_gpu_color_dict_99021f5e376737f8: function(arg0, arg1) {
            getObject(arg0).clearValue = getObject(arg1);
        },
        __wbg_set_code_e21341a422769059: function(arg0, arg1, arg2) {
            getObject(arg0).code = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_color_52712a5762e024a5: function(arg0, arg1) {
            getObject(arg0).color = getObject(arg1);
        },
        __wbg_set_color_attachments_5f259d10d3f3ccb9: function(arg0, arg1, arg2) {
            getObject(arg0).colorAttachments = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_compare_3bf121787fa29957: function(arg0, arg1) {
            getObject(arg0).compare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_compare_8dfcf70fcd8cfb94: function(arg0, arg1) {
            getObject(arg0).compare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_count_7d1a50f691560ead: function(arg0, arg1) {
            getObject(arg0).count = arg1 >>> 0;
        },
        __wbg_set_credentials_bf1d3e014213a571: function(arg0, arg1) {
            getObject(arg0).credentials = __wbindgen_enum_RequestCredentials[arg1];
        },
        __wbg_set_cull_mode_7adceafd1445c2a6: function(arg0, arg1) {
            getObject(arg0).cullMode = __wbindgen_enum_GpuCullMode[arg1];
        },
        __wbg_set_depth_bias_2ff83b7698756d98: function(arg0, arg1) {
            getObject(arg0).depthBias = arg1;
        },
        __wbg_set_depth_bias_clamp_c47db91f0dc154bc: function(arg0, arg1) {
            getObject(arg0).depthBiasClamp = arg1;
        },
        __wbg_set_depth_bias_slope_scale_1d54ca553d61de73: function(arg0, arg1) {
            getObject(arg0).depthBiasSlopeScale = arg1;
        },
        __wbg_set_depth_clear_value_6d54e630021b8d0a: function(arg0, arg1) {
            getObject(arg0).depthClearValue = arg1;
        },
        __wbg_set_depth_compare_7907c518ffb9c07a: function(arg0, arg1) {
            getObject(arg0).depthCompare = __wbindgen_enum_GpuCompareFunction[arg1];
        },
        __wbg_set_depth_fail_op_287eae3d77fec685: function(arg0, arg1) {
            getObject(arg0).depthFailOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_depth_load_op_66e18cce0adc6d04: function(arg0, arg1) {
            getObject(arg0).depthLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_depth_or_array_layers_b977da6f11e5640d: function(arg0, arg1) {
            getObject(arg0).depthOrArrayLayers = arg1 >>> 0;
        },
        __wbg_set_depth_read_only_9107ee4dd8709efb: function(arg0, arg1) {
            getObject(arg0).depthReadOnly = arg1 !== 0;
        },
        __wbg_set_depth_stencil_9789642fce399621: function(arg0, arg1) {
            getObject(arg0).depthStencil = getObject(arg1);
        },
        __wbg_set_depth_stencil_attachment_42e5e53fa0739216: function(arg0, arg1) {
            getObject(arg0).depthStencilAttachment = getObject(arg1);
        },
        __wbg_set_depth_store_op_be93b0ac5d45803f: function(arg0, arg1) {
            getObject(arg0).depthStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_depth_write_enabled_ae5d4ffd893432e4: function(arg0, arg1) {
            getObject(arg0).depthWriteEnabled = arg1 !== 0;
        },
        __wbg_set_device_603f685db59c1649: function(arg0, arg1) {
            getObject(arg0).device = getObject(arg1);
        },
        __wbg_set_dimension_13da043f59bc977f: function(arg0, arg1) {
            getObject(arg0).dimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_dimension_443383e1f360a896: function(arg0, arg1) {
            getObject(arg0).dimension = __wbindgen_enum_GpuTextureDimension[arg1];
        },
        __wbg_set_dst_factor_16e250bd2f5f8a77: function(arg0, arg1) {
            getObject(arg0).dstFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        },
        __wbg_set_end_of_pass_write_index_61a9cd23380259bd: function(arg0, arg1) {
            getObject(arg0).endOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_entries_7dfa89a00bdff489: function(arg0, arg1, arg2) {
            getObject(arg0).entries = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_entries_f60b26345b2c7977: function(arg0, arg1, arg2) {
            getObject(arg0).entries = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_40a004953671c1a0: function(arg0, arg1, arg2) {
            getObject(arg0).entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_571795ff6bd3e5bc: function(arg0, arg1, arg2) {
            getObject(arg0).entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_external_texture_bdd5ebdc331c0d06: function(arg0, arg1) {
            getObject(arg0).externalTexture = getObject(arg1);
        },
        __wbg_set_fail_op_dcbcb832b22acb0d: function(arg0, arg1) {
            getObject(arg0).failOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_fillStyle_78a9c646f2b2b336: function(arg0, arg1, arg2) {
            getObject(arg0).fillStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_font_d494a5a1edb8cff8: function(arg0, arg1, arg2) {
            getObject(arg0).font = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_format_4ae9d5555ecce919: function(arg0, arg1) {
            getObject(arg0).format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_5444425dee1be1ea: function(arg0, arg1) {
            getObject(arg0).format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_586edf6514c11751: function(arg0, arg1) {
            getObject(arg0).format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_617abf73e2d5bc21: function(arg0, arg1) {
            getObject(arg0).format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_9272f72a7f2c0700: function(arg0, arg1) {
            getObject(arg0).format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_format_d607f5bd6106e2fc: function(arg0, arg1) {
            getObject(arg0).format = __wbindgen_enum_GpuVertexFormat[arg1];
        },
        __wbg_set_format_d916525ae7d18e88: function(arg0, arg1) {
            getObject(arg0).format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_fragment_788168872bc7434a: function(arg0, arg1) {
            getObject(arg0).fragment = getObject(arg1);
        },
        __wbg_set_front_face_ab616b7a790ea568: function(arg0, arg1) {
            getObject(arg0).frontFace = __wbindgen_enum_GpuFrontFace[arg1];
        },
        __wbg_set_g_b5ea7c69d5f86113: function(arg0, arg1) {
            getObject(arg0).g = arg1;
        },
        __wbg_set_has_dynamic_offset_ccafd64f17d0552b: function(arg0, arg1) {
            getObject(arg0).hasDynamicOffset = arg1 !== 0;
        },
        __wbg_set_height_1202ad0eab43cbe0: function(arg0, arg1) {
            getObject(arg0).height = arg1 >>> 0;
        },
        __wbg_set_height_1c5337b96202bcec: function(arg0, arg1) {
            getObject(arg0).height = arg1 >>> 0;
        },
        __wbg_set_height_698fb3b255bc1348: function(arg0, arg1) {
            getObject(arg0).height = arg1 >>> 0;
        },
        __wbg_set_label_1804369b72a4e735: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_346bf11b8ec29fea: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_3ac34d6a447c14b6: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_48cf960febfa9f8b: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_4ff277997e6b21ec: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_59ba23a4c997818b: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_5bd53d250b9493ed: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_83d1c79a73f44a36: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_8a971b985701494c: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_8bef035fed27f91a: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_951186a6d8fcd032: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_ec5af0e2c24eb6d3: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_fe4b30a50c9ac9c2: function(arg0, arg1, arg2) {
            getObject(arg0).label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_layout_75642eb58293c075: function(arg0, arg1) {
            getObject(arg0).layout = getObject(arg1);
        },
        __wbg_set_layout_94658faf9a0be06d: function(arg0, arg1) {
            getObject(arg0).layout = getObject(arg1);
        },
        __wbg_set_layout_gpu_auto_layout_mode_4ee68bed4b80a76e: function(arg0, arg1) {
            getObject(arg0).layout = __wbindgen_enum_GpuAutoLayoutMode[arg1];
        },
        __wbg_set_load_op_18749bb4ddd7ea87: function(arg0, arg1) {
            getObject(arg0).loadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_lod_max_clamp_fef0518bc3ead2d3: function(arg0, arg1) {
            getObject(arg0).lodMaxClamp = arg1;
        },
        __wbg_set_lod_min_clamp_350395dcc004a30c: function(arg0, arg1) {
            getObject(arg0).lodMinClamp = arg1;
        },
        __wbg_set_mag_filter_dcaf3efc70dadf8b: function(arg0, arg1) {
            getObject(arg0).magFilter = __wbindgen_enum_GpuFilterMode[arg1];
        },
        __wbg_set_mapped_at_creation_f771fc107b1310a5: function(arg0, arg1) {
            getObject(arg0).mappedAtCreation = arg1 !== 0;
        },
        __wbg_set_mask_7963aed8bfc37ee8: function(arg0, arg1) {
            getObject(arg0).mask = arg1 >>> 0;
        },
        __wbg_set_max_anisotropy_a5ad085256586d87: function(arg0, arg1) {
            getObject(arg0).maxAnisotropy = arg1;
        },
        __wbg_set_method_82e6c3c083da0734: function(arg0, arg1, arg2) {
            getObject(arg0).method = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_min_binding_size_f64_74baaf5086ba50e2: function(arg0, arg1) {
            getObject(arg0).minBindingSize = arg1;
        },
        __wbg_set_min_filter_d78cdacbc0f7dd89: function(arg0, arg1) {
            getObject(arg0).minFilter = __wbindgen_enum_GpuFilterMode[arg1];
        },
        __wbg_set_mip_level_60bb0b4f30d8147f: function(arg0, arg1) {
            getObject(arg0).mipLevel = arg1 >>> 0;
        },
        __wbg_set_mip_level_count_05d78552d6bb9c0b: function(arg0, arg1) {
            getObject(arg0).mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mip_level_count_7be98f67608cff20: function(arg0, arg1) {
            getObject(arg0).mipLevelCount = arg1 >>> 0;
        },
        __wbg_set_mipmap_filter_eed32e31e8eb5a81: function(arg0, arg1) {
            getObject(arg0).mipmapFilter = __wbindgen_enum_GpuMipmapFilterMode[arg1];
        },
        __wbg_set_mode_cd0f311d22e1087e: function(arg0, arg1) {
            getObject(arg0).mode = __wbindgen_enum_GpuCanvasToneMappingMode[arg1];
        },
        __wbg_set_module_131df59380b6a8c1: function(arg0, arg1) {
            getObject(arg0).module = getObject(arg1);
        },
        __wbg_set_module_4626f14c145a0bee: function(arg0, arg1) {
            getObject(arg0).module = getObject(arg1);
        },
        __wbg_set_multisample_95e56b50ee346453: function(arg0, arg1) {
            getObject(arg0).multisample = getObject(arg1);
        },
        __wbg_set_multisampled_82f133b97717fed0: function(arg0, arg1) {
            getObject(arg0).multisampled = arg1 !== 0;
        },
        __wbg_set_name_b8b5bb14410a80d5: function(arg0, arg1, arg2) {
            getObject(arg0).name = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_offset_f64_a2781064c004de37: function(arg0, arg1) {
            getObject(arg0).offset = arg1;
        },
        __wbg_set_offset_f64_b9040b7b4b3b58ef: function(arg0, arg1) {
            getObject(arg0).offset = arg1;
        },
        __wbg_set_offset_f64_f6c19c7eab3139c3: function(arg0, arg1) {
            getObject(arg0).offset = arg1;
        },
        __wbg_set_onmessage_d7e69fe35723df1e: function(arg0, arg1) {
            getObject(arg0).onmessage = getObject(arg1);
        },
        __wbg_set_onuncapturederror_47c12cc2f5c09a08: function(arg0, arg1) {
            getObject(arg0).onuncapturederror = getObject(arg1);
        },
        __wbg_set_operation_6e3e9951bf90ea60: function(arg0, arg1) {
            getObject(arg0).operation = __wbindgen_enum_GpuBlendOperation[arg1];
        },
        __wbg_set_origin_gpu_origin_3d_dict_bcab737ae5849293: function(arg0, arg1) {
            getObject(arg0).origin = getObject(arg1);
        },
        __wbg_set_pass_op_e450050db694810b: function(arg0, arg1) {
            getObject(arg0).passOp = __wbindgen_enum_GpuStencilOperation[arg1];
        },
        __wbg_set_power_preference_08fe83a5eadb70d0: function(arg0, arg1) {
            getObject(arg0).powerPreference = __wbindgen_enum_GpuPowerPreference[arg1];
        },
        __wbg_set_prevent_scroll_046a03f15b24923e: function(arg0, arg1) {
            getObject(arg0).preventScroll = arg1 !== 0;
        },
        __wbg_set_primitive_c7b06d4b7e532d79: function(arg0, arg1) {
            getObject(arg0).primitive = getObject(arg1);
        },
        __wbg_set_query_set_ec44c9644d5fb7aa: function(arg0, arg1) {
            getObject(arg0).querySet = getObject(arg1);
        },
        __wbg_set_r_f315342365057bfd: function(arg0, arg1) {
            getObject(arg0).r = arg1;
        },
        __wbg_set_readOnly_cc21f7fc21a056b4: function(arg0, arg1) {
            getObject(arg0).readOnly = arg1 !== 0;
        },
        __wbg_set_redirect_96061705212f7800: function(arg0, arg1) {
            getObject(arg0).redirect = __wbindgen_enum_RequestRedirect[arg1];
        },
        __wbg_set_required_features_a03feb65576a939d: function(arg0, arg1, arg2) {
            getObject(arg0).requiredFeatures = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_required_limits_715c4f61cab45cbd: function(arg0, arg1) {
            getObject(arg0).requiredLimits = getObject(arg1);
        },
        __wbg_set_resolve_target_gpu_texture_view_596d364279303c36: function(arg0, arg1) {
            getObject(arg0).resolveTarget = getObject(arg1);
        },
        __wbg_set_resource_8535e18d45831050: function(arg0, arg1) {
            getObject(arg0).resource = getObject(arg1);
        },
        __wbg_set_resource_gpu_buffer_binding_5450144e7744c01d: function(arg0, arg1) {
            getObject(arg0).resource = getObject(arg1);
        },
        __wbg_set_resource_gpu_texture_view_18f08044da0f00bb: function(arg0, arg1) {
            getObject(arg0).resource = getObject(arg1);
        },
        __wbg_set_rows_per_image_7cf25f1f6da0dbf0: function(arg0, arg1) {
            getObject(arg0).rowsPerImage = arg1 >>> 0;
        },
        __wbg_set_sample_count_2698a73135c00d4c: function(arg0, arg1) {
            getObject(arg0).sampleCount = arg1 >>> 0;
        },
        __wbg_set_sample_type_18ebe049de8b818f: function(arg0, arg1) {
            getObject(arg0).sampleType = __wbindgen_enum_GpuTextureSampleType[arg1];
        },
        __wbg_set_sampler_5d6e781457aaf9ac: function(arg0, arg1) {
            getObject(arg0).sampler = getObject(arg1);
        },
        __wbg_set_shader_location_1aa266e5091a4621: function(arg0, arg1) {
            getObject(arg0).shaderLocation = arg1 >>> 0;
        },
        __wbg_set_size_f64_1de16d25a7c2d7db: function(arg0, arg1) {
            getObject(arg0).size = arg1;
        },
        __wbg_set_size_f64_6db4344b19dff88b: function(arg0, arg1) {
            getObject(arg0).size = arg1;
        },
        __wbg_set_size_gpu_extent_3d_dict_fda5a4953e4d53d2: function(arg0, arg1) {
            getObject(arg0).size = getObject(arg1);
        },
        __wbg_set_spellcheck_63cf65835a89fcb4: function(arg0, arg1) {
            getObject(arg0).spellcheck = arg1 !== 0;
        },
        __wbg_set_src_factor_b9dea09afc88ffb7: function(arg0, arg1) {
            getObject(arg0).srcFactor = __wbindgen_enum_GpuBlendFactor[arg1];
        },
        __wbg_set_stencil_back_e7d5a419c46a2fc7: function(arg0, arg1) {
            getObject(arg0).stencilBack = getObject(arg1);
        },
        __wbg_set_stencil_clear_value_b3fb1a1021fa2bd0: function(arg0, arg1) {
            getObject(arg0).stencilClearValue = arg1 >>> 0;
        },
        __wbg_set_stencil_front_4bfa06cae0a38cf4: function(arg0, arg1) {
            getObject(arg0).stencilFront = getObject(arg1);
        },
        __wbg_set_stencil_load_op_23472dd8295389a5: function(arg0, arg1) {
            getObject(arg0).stencilLoadOp = __wbindgen_enum_GpuLoadOp[arg1];
        },
        __wbg_set_stencil_read_mask_d5e64669f0e42581: function(arg0, arg1) {
            getObject(arg0).stencilReadMask = arg1 >>> 0;
        },
        __wbg_set_stencil_read_only_641c611c5324d3a6: function(arg0, arg1) {
            getObject(arg0).stencilReadOnly = arg1 !== 0;
        },
        __wbg_set_stencil_store_op_2a18c3c02cbdd402: function(arg0, arg1) {
            getObject(arg0).stencilStoreOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_stencil_write_mask_3da7a76a556cae24: function(arg0, arg1) {
            getObject(arg0).stencilWriteMask = arg1 >>> 0;
        },
        __wbg_set_step_mode_d92d664d27163aab: function(arg0, arg1) {
            getObject(arg0).stepMode = __wbindgen_enum_GpuVertexStepMode[arg1];
        },
        __wbg_set_storage_texture_dccfcb6d8685a28b: function(arg0, arg1) {
            getObject(arg0).storageTexture = getObject(arg1);
        },
        __wbg_set_store_op_8847746a474253c8: function(arg0, arg1) {
            getObject(arg0).storeOp = __wbindgen_enum_GpuStoreOp[arg1];
        },
        __wbg_set_strip_index_format_7a32ee9bf6f4f18f: function(arg0, arg1) {
            getObject(arg0).stripIndexFormat = __wbindgen_enum_GpuIndexFormat[arg1];
        },
        __wbg_set_tabIndex_35927a28c713d094: function(arg0, arg1) {
            getObject(arg0).tabIndex = arg1;
        },
        __wbg_set_targets_53333081ad7283b3: function(arg0, arg1, arg2) {
            getObject(arg0).targets = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_textAlign_8f44e2a0c90bef9a: function(arg0, arg1, arg2) {
            getObject(arg0).textAlign = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textBaseline_b0127279ac0d10dd: function(arg0, arg1, arg2) {
            getObject(arg0).textBaseline = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textContent_729c78ca859aa5a5: function(arg0, arg1, arg2) {
            getObject(arg0).textContent = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_texture_60283d5a3509281c: function(arg0, arg1) {
            getObject(arg0).texture = getObject(arg1);
        },
        __wbg_set_texture_893b0a155eac7bb3: function(arg0, arg1) {
            getObject(arg0).texture = getObject(arg1);
        },
        __wbg_set_timeout_8e970e0deb736eb0: function(arg0, arg1) {
            getObject(arg0).timeout = arg1 >>> 0;
        },
        __wbg_set_timestamp_writes_5e040caa4c5ec65c: function(arg0, arg1) {
            getObject(arg0).timestampWrites = getObject(arg1);
        },
        __wbg_set_title_576991c4a4b3c882: function(arg0, arg1, arg2) {
            getObject(arg0).title = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_tone_mapping_ae4cebc9871e0e30: function(arg0, arg1) {
            getObject(arg0).toneMapping = getObject(arg1);
        },
        __wbg_set_topology_b009640c64f39f91: function(arg0, arg1) {
            getObject(arg0).topology = __wbindgen_enum_GpuPrimitiveTopology[arg1];
        },
        __wbg_set_type_87c1847bf338c3cf: function(arg0, arg1) {
            getObject(arg0).type = __wbindgen_enum_GpuBufferBindingType[arg1];
        },
        __wbg_set_type_e3051f91e1914ac5: function(arg0, arg1) {
            getObject(arg0).type = __wbindgen_enum_GpuSamplerBindingType[arg1];
        },
        __wbg_set_type_f73a726d9729f489: function(arg0, arg1) {
            getObject(arg0).type = __wbindgen_enum_WorkerType[arg1];
        },
        __wbg_set_unclipped_depth_011df4d1ed8c4a83: function(arg0, arg1) {
            getObject(arg0).unclippedDepth = arg1 !== 0;
        },
        __wbg_set_usage_29b8c421aa91de96: function(arg0, arg1) {
            getObject(arg0).usage = arg1 >>> 0;
        },
        __wbg_set_usage_79b5edecf249b5a0: function(arg0, arg1) {
            getObject(arg0).usage = arg1 >>> 0;
        },
        __wbg_set_usage_95369f4e048501d2: function(arg0, arg1) {
            getObject(arg0).usage = arg1 >>> 0;
        },
        __wbg_set_usage_f7fa258e2db2a3be: function(arg0, arg1) {
            getObject(arg0).usage = arg1 >>> 0;
        },
        __wbg_set_value_6eb2c83ae62c950f: function(arg0, arg1, arg2) {
            getObject(arg0).value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_vertex_d0c223291efc6077: function(arg0, arg1) {
            getObject(arg0).vertex = getObject(arg1);
        },
        __wbg_set_view_dimension_79c4ba7d05ee0f18: function(arg0, arg1) {
            getObject(arg0).viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_view_dimension_c8a3183d873d8457: function(arg0, arg1) {
            getObject(arg0).viewDimension = __wbindgen_enum_GpuTextureViewDimension[arg1];
        },
        __wbg_set_view_formats_382ea2030d170c54: function(arg0, arg1, arg2) {
            getObject(arg0).viewFormats = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_view_formats_6fcf6f94ff60bd4c: function(arg0, arg1, arg2) {
            getObject(arg0).viewFormats = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_view_gpu_texture_view_3a5332cb959dc2ec: function(arg0, arg1) {
            getObject(arg0).view = getObject(arg1);
        },
        __wbg_set_view_gpu_texture_view_e9f3236d28582143: function(arg0, arg1) {
            getObject(arg0).view = getObject(arg1);
        },
        __wbg_set_visibility_13119ae63ddb4d54: function(arg0, arg1) {
            getObject(arg0).visibility = arg1 >>> 0;
        },
        __wbg_set_width_4c3a2252e0dea033: function(arg0, arg1) {
            getObject(arg0).width = arg1 >>> 0;
        },
        __wbg_set_width_c854f1a0cc753050: function(arg0, arg1) {
            getObject(arg0).width = arg1 >>> 0;
        },
        __wbg_set_width_f52a20e39808b138: function(arg0, arg1) {
            getObject(arg0).width = arg1 >>> 0;
        },
        __wbg_set_write_mask_b58f1a07dcba6420: function(arg0, arg1) {
            getObject(arg0).writeMask = arg1 >>> 0;
        },
        __wbg_set_x_c6eb18f8be4cfdfa: function(arg0, arg1) {
            getObject(arg0).x = arg1 >>> 0;
        },
        __wbg_set_y_47ec164aaba8896b: function(arg0, arg1) {
            getObject(arg0).y = arg1 >>> 0;
        },
        __wbg_set_z_bef3955d95557928: function(arg0, arg1) {
            getObject(arg0).z = arg1 >>> 0;
        },
        __wbg_shaderSource_108c27c8694ef4c0: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
        },
        __wbg_shaderSource_82a2cbfe223a86e0: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).shaderSource(getObject(arg1), getStringFromWasm0(arg2, arg3));
        },
        __wbg_shiftKey_2b651c3e5a9c07f5: function(arg0) {
            const ret = getObject(arg0).shiftKey;
            return ret;
        },
        __wbg_shiftKey_9dd20e0fb9da6eb5: function(arg0) {
            const ret = getObject(arg0).shiftKey;
            return ret;
        },
        __wbg_size_338f1717fbf84c4e: function(arg0) {
            const ret = getObject(arg0).size;
            return ret;
        },
        __wbg_slice_43ecce137499fbc3: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = getObject(arg0).slice(arg1, arg2, getStringFromWasm0(arg3, arg4));
            return addHeapObject(ret);
        }, arguments); },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = getObject(arg1).stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_8eb4cd83130a11a0: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_1e7044f654e934db: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_SELF_d8b50611246a6d92: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_static_accessor_WINDOW_fd0bc376bf0f8b42: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_status_88ccc72fe7bea621: function(arg0) {
            const ret = getObject(arg0).status;
            return ret;
        },
        __wbg_stencilFuncSeparate_d7b5956ce3fe1272: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).stencilFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3, arg4 >>> 0);
        },
        __wbg_stencilFuncSeparate_fd17ac8961fd8359: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).stencilFuncSeparate(arg1 >>> 0, arg2 >>> 0, arg3, arg4 >>> 0);
        },
        __wbg_stencilMaskSeparate_5f3823dfe31289be: function(arg0, arg1, arg2) {
            getObject(arg0).stencilMaskSeparate(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_stencilMaskSeparate_d0910e6d55b4605a: function(arg0, arg1, arg2) {
            getObject(arg0).stencilMaskSeparate(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_stencilMask_4a8c4bda850006f1: function(arg0, arg1) {
            getObject(arg0).stencilMask(arg1 >>> 0);
        },
        __wbg_stencilMask_cb69051e26a265bb: function(arg0, arg1) {
            getObject(arg0).stencilMask(arg1 >>> 0);
        },
        __wbg_stencilOpSeparate_3fe9feaf978f0a17: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).stencilOpSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_stencilOpSeparate_cb0bb47587f022de: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).stencilOpSeparate(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0, arg4 >>> 0);
        },
        __wbg_store_7b84f2139ed609ba: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Atomics.store(getObject(arg0), arg1 >>> 0, arg2);
            return ret;
        }, arguments); },
        __wbg_style_b79c69cee0c004b9: function(arg0) {
            const ret = getObject(arg0).style;
            return addHeapObject(ret);
        },
        __wbg_subgroupMaxSize_1317a597b4caba08: function(arg0) {
            const ret = getObject(arg0).subgroupMaxSize;
            return ret;
        },
        __wbg_subgroupMinSize_ba60776f25dadbfa: function(arg0) {
            const ret = getObject(arg0).subgroupMinSize;
            return ret;
        },
        __wbg_submit_c4379e221512d3fa: function(arg0, arg1, arg2) {
            getObject(arg0).submit(getArrayJsValueViewFromWasm0(arg1, arg2));
        },
        __wbg_texImage2D_1b8d273f15622f0e: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texImage2D_6fca8c58ea7e4c8e: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_texImage2D_8c5d6403fa51874d: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texImage3D_2bba05e03e9def07: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            getObject(arg0).texImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8 >>> 0, arg9 >>> 0, getObject(arg10));
        }, arguments); },
        __wbg_texImage3D_3977c422ce82cf99: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10) {
            getObject(arg0).texImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8 >>> 0, arg9 >>> 0, arg10);
        }, arguments); },
        __wbg_texParameteri_1fea50207adbfd68: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        },
        __wbg_texParameteri_e127aa385b821d68: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).texParameteri(arg1 >>> 0, arg2 >>> 0, arg3);
        },
        __wbg_texStorage2D_eb27b3383d99c349: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).texStorage2D(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        },
        __wbg_texStorage3D_cb53309842f2c8eb: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).texStorage3D(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5, arg6);
        },
        __wbg_texSubImage2D_0542ae723245dd04: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, arg9);
        }, arguments); },
        __wbg_texSubImage2D_0721d9d910ecf097: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texSubImage2D_11c39463147c9d38: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texSubImage2D_21b4316b692aa7bc: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texSubImage2D_339324fe86d9879e: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texSubImage2D_6218009ef1e27c0e: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texSubImage2D_79b4ac26a5580960: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texSubImage2D_892cfa51f196ee93: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9) {
            getObject(arg0).texSubImage2D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7 >>> 0, arg8 >>> 0, getObject(arg9));
        }, arguments); },
        __wbg_texSubImage3D_6458575ddb6df6b3: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, arg11);
        }, arguments); },
        __wbg_texSubImage3D_747bc81ed2ed4116: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, getObject(arg11));
        }, arguments); },
        __wbg_texSubImage3D_7a674dd9403f8be3: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, getObject(arg11));
        }, arguments); },
        __wbg_texSubImage3D_805532d56bdd814d: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, getObject(arg11));
        }, arguments); },
        __wbg_texSubImage3D_ad76ff2925daf174: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, getObject(arg11));
        }, arguments); },
        __wbg_texSubImage3D_c65a616189ed3b19: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, getObject(arg11));
        }, arguments); },
        __wbg_texSubImage3D_f99458987d5b061a: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9, arg10, arg11) {
            getObject(arg0).texSubImage3D(arg1 >>> 0, arg2, arg3, arg4, arg5, arg6, arg7, arg8, arg9 >>> 0, arg10 >>> 0, getObject(arg11));
        }, arguments); },
        __wbg_text_2c72c05ea82efae7: function(arg0) {
            const ret = getObject(arg0).text();
            return addHeapObject(ret);
        },
        __wbg_then_7a850dae4493f353: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
            return addHeapObject(ret);
        },
        __wbg_then_8ebf7f4d2b402f98: function(arg0, arg1) {
            const ret = getObject(arg0).then(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_then_b830475380919203: function(arg0, arg1) {
            const ret = getObject(arg0).then(getObject(arg1));
            return addHeapObject(ret);
        },
        __wbg_timeRemaining_cf7907409998546f: function(arg0) {
            const ret = getObject(arg0).timeRemaining();
            return ret;
        },
        __wbg_timeStamp_0f2dfa63a941bc70: function(arg0) {
            const ret = getObject(arg0).timeStamp;
            return ret;
        },
        __wbg_top_319e8c7a40daacf1: function(arg0) {
            const ret = getObject(arg0).top;
            return ret;
        },
        __wbg_type_b801ad2eeaceaba5: function(arg0, arg1) {
            const ret = getObject(arg1).type;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_types_03ccd563e1a1981f: function(arg0) {
            const ret = getObject(arg0).types;
            return addHeapObject(ret);
        },
        __wbg_unconfigure_7ca436ff70c0bcc3: function(arg0) {
            getObject(arg0).unconfigure();
        },
        __wbg_uniform1f_6067a9c69ed38c8b: function(arg0, arg1, arg2) {
            getObject(arg0).uniform1f(getObject(arg1), arg2);
        },
        __wbg_uniform1f_d08261cd87e412e0: function(arg0, arg1, arg2) {
            getObject(arg0).uniform1f(getObject(arg1), arg2);
        },
        __wbg_uniform1i_25a3df9f69c2f765: function(arg0, arg1, arg2) {
            getObject(arg0).uniform1i(getObject(arg1), arg2);
        },
        __wbg_uniform1i_89c631d199860f60: function(arg0, arg1, arg2) {
            getObject(arg0).uniform1i(getObject(arg1), arg2);
        },
        __wbg_uniform1ui_58cbbe1a93e44a02: function(arg0, arg1, arg2) {
            getObject(arg0).uniform1ui(getObject(arg1), arg2 >>> 0);
        },
        __wbg_uniform2fv_6a494ce09154afdb: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2fv(getObject(arg1), getArrayF32FromWasm0(arg2, arg3));
        },
        __wbg_uniform2fv_f85b3b2779a6c2b9: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2fv(getObject(arg1), getArrayF32FromWasm0(arg2, arg3));
        },
        __wbg_uniform2iv_5c591a2edfd26c45: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2iv(getObject(arg1), getArrayI32FromWasm0(arg2, arg3));
        },
        __wbg_uniform2iv_63a2ba7206eaf2b3: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2iv(getObject(arg1), getArrayI32FromWasm0(arg2, arg3));
        },
        __wbg_uniform2uiv_aaa076c2f50b5c86: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform2uiv(getObject(arg1), getArrayU32FromWasm0(arg2, arg3));
        },
        __wbg_uniform3fv_2e139d144894f1ec: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform3fv(getObject(arg1), getArrayF32FromWasm0(arg2, arg3));
        },
        __wbg_uniform3fv_dfaba6b65cfc2c2b: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform3fv(getObject(arg1), getArrayF32FromWasm0(arg2, arg3));
        },
        __wbg_uniform3iv_700228c611ca323d: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform3iv(getObject(arg1), getArrayI32FromWasm0(arg2, arg3));
        },
        __wbg_uniform3iv_f32512d7179442f8: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform3iv(getObject(arg1), getArrayI32FromWasm0(arg2, arg3));
        },
        __wbg_uniform3uiv_c972de1ee5eb2b48: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform3uiv(getObject(arg1), getArrayU32FromWasm0(arg2, arg3));
        },
        __wbg_uniform4f_b5ea01e0388c425e: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).uniform4f(getObject(arg1), arg2, arg3, arg4, arg5);
        },
        __wbg_uniform4f_b9880b3e743b1cf3: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).uniform4f(getObject(arg1), arg2, arg3, arg4, arg5);
        },
        __wbg_uniform4fv_0763a5cfb3d561c9: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform4fv(getObject(arg1), getArrayF32FromWasm0(arg2, arg3));
        },
        __wbg_uniform4fv_95b1afba56301916: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform4fv(getObject(arg1), getArrayF32FromWasm0(arg2, arg3));
        },
        __wbg_uniform4iv_1269cd3a3f01909d: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform4iv(getObject(arg1), getArrayI32FromWasm0(arg2, arg3));
        },
        __wbg_uniform4iv_84dc6c0b2ea09256: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform4iv(getObject(arg1), getArrayI32FromWasm0(arg2, arg3));
        },
        __wbg_uniform4uiv_13ca9c250b538d85: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniform4uiv(getObject(arg1), getArrayU32FromWasm0(arg2, arg3));
        },
        __wbg_uniformBlockBinding_d97dbfc1886608c9: function(arg0, arg1, arg2, arg3) {
            getObject(arg0).uniformBlockBinding(getObject(arg1), arg2 >>> 0, arg3 >>> 0);
        },
        __wbg_uniformMatrix2fv_4a95dd9315ceafce: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix2fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix2fv_554c771038fd18e7: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix2fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix2x3fv_18f2ff07b55f7458: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix2x3fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix2x4fv_2b66cdeb52233873: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix2x4fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix3fv_303d7eef53df7ed2: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix3fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix3fv_a9db97f34b3706b3: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix3fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix3x2fv_67c65614f0e122e8: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix3x2fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix3x4fv_e3bd46b1bf5227ed: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix3x4fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix4fv_db6f9ad4f00e3b76: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix4fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix4fv_e411faf4de6021c5: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix4fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix4x2fv_c59133281828fe8e: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix4x2fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_uniformMatrix4x3fv_dc85828b88505234: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).uniformMatrix4x3fv(getObject(arg1), arg2 !== 0, getArrayF32FromWasm0(arg3, arg4));
        },
        __wbg_unobserve_a811aa8fa2d41747: function(arg0, arg1) {
            getObject(arg0).unobserve(getObject(arg1));
        },
        __wbg_useProgram_7019ac9ef4bdbe96: function(arg0, arg1) {
            getObject(arg0).useProgram(getObject(arg1));
        },
        __wbg_useProgram_fcab4319331e0094: function(arg0, arg1) {
            getObject(arg0).useProgram(getObject(arg1));
        },
        __wbg_userAgent_8f029431f94bc023: function() { return handleError(function (arg0, arg1) {
            const ret = getObject(arg1).userAgent;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_valueOf_24fc886d85bf55b5: function(arg0) {
            const ret = getObject(arg0).valueOf();
            return addHeapObject(ret);
        },
        __wbg_value_000080a4486e3ed3: function(arg0, arg1) {
            const ret = getObject(arg1).value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export, wasm.__wbindgen_export2);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_vertexAttribDivisorANGLE_5bebd4490e25d0f4: function(arg0, arg1, arg2) {
            getObject(arg0).vertexAttribDivisorANGLE(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_vertexAttribDivisor_e2a9ba8c397af61b: function(arg0, arg1, arg2) {
            getObject(arg0).vertexAttribDivisor(arg1 >>> 0, arg2 >>> 0);
        },
        __wbg_vertexAttribIPointer_660e209bed7575ab: function(arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).vertexAttribIPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4, arg5);
        },
        __wbg_vertexAttribPointer_b6db7a62b9bda090: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        },
        __wbg_vertexAttribPointer_ed019b681b1aaacf: function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).vertexAttribPointer(arg1 >>> 0, arg2, arg3 >>> 0, arg4 !== 0, arg5, arg6);
        },
        __wbg_view_d8c7b26e4d4650f1: function(arg0) {
            const ret = getObject(arg0).view;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_viewport_704a6159fb0395cb: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).viewport(arg1, arg2, arg3, arg4);
        },
        __wbg_viewport_f9d3214251b38c39: function(arg0, arg1, arg2, arg3, arg4) {
            getObject(arg0).viewport(arg1, arg2, arg3, arg4);
        },
        __wbg_visualViewport_11fba25a415e79fa: function(arg0) {
            const ret = getObject(arg0).visualViewport;
            return isLikeNone(ret) ? 0 : addHeapObject(ret);
        },
        __wbg_waitAsync_fab5b4fe4c49854c: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Atomics.waitAsync(getObject(arg0), arg1 >>> 0, arg2);
            return addHeapObject(ret);
        }, arguments); },
        __wbg_warn_d3544c7814fab534: function(arg0) {
            console.warn(getObject(arg0));
        },
        __wbg_width_0fe8999be2c6f812: function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        },
        __wbg_width_66ae60ec9345f56c: function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        },
        __wbg_width_71b671f940e3eb8c: function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        },
        __wbg_width_9293d61efea033e6: function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        },
        __wbg_width_b0c7f2ae9ce3ffb0: function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        },
        __wbg_width_b5e609025d3f7451: function(arg0) {
            const ret = getObject(arg0).width;
            return ret;
        },
        __wbg_width_caff1500288aaffa: function() { return handleError(function (arg0) {
            const ret = getObject(arg0).width;
            return ret;
        }, arguments); },
        __wbg_writeBuffer_badef58fae6c9543: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            getObject(arg0).writeBuffer(getObject(arg1), arg2, getArrayU8FromWasm0(arg3, arg4), arg5, arg6);
        }, arguments); },
        __wbg_writeText_efbdad9c92eb55b9: function(arg0, arg1, arg2) {
            const ret = getObject(arg0).writeText(getStringFromWasm0(arg1, arg2));
            return addHeapObject(ret);
        },
        __wbg_writeTexture_4d444c92b271de25: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            getObject(arg0).writeTexture(getObject(arg1), getArrayU8FromWasm0(arg2, arg3), getObject(arg4), getObject(arg5));
        }, arguments); },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 11817, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_26662);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 3241, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_5369);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Array<any>")], shim_idx: 3241, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_5369_19);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("GPUDevice")], shim_idx: 3876, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_6284);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("GPUDeviceLostInfo")], shim_idx: 3880, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_6288);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("GPUUncapturedErrorEvent")], shim_idx: 3880, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_6288_22);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("IdleDeadline")], shim_idx: 3241, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_5369_23);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000008: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("any")], shim_idx: 3876, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_6284_24);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000009: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("undefined")], shim_idx: 3876, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_6284_25);
            return addHeapObject(ret);
        },
        __wbindgen_generic_000000000000000a: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Ref(NamedExternref("MessageEvent"))], shim_idx: 11590, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_24000);
            return addHeapObject(ret);
        },
        __wbindgen_generic_000000000000000b: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 3245, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, __wasm_bindgen_func_elem_5373);
            return addHeapObject(ret);
        },
        __wbindgen_generic_000000000000000c: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return addHeapObject(ret);
        },
        __wbindgen_generic_000000000000000d: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(F32)) -> NamedExternref("Float32Array")`.
            const ret = getArrayF32FromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_generic_000000000000000e: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(I16)) -> NamedExternref("Int16Array")`.
            const ret = getArrayI16FromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_generic_000000000000000f: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(I32)) -> NamedExternref("Int32Array")`.
            const ret = getArrayI32FromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000010: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(I8)) -> NamedExternref("Int8Array")`.
            const ret = getArrayI8FromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000011: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U16)) -> NamedExternref("Uint16Array")`.
            const ret = getArrayU16FromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000012: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U32)) -> NamedExternref("Uint32Array")`.
            const ret = getArrayU32FromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000013: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_generic_0000000000000014: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_object_clone_ref: function(arg0) {
            const ret = getObject(arg0);
            return addHeapObject(ret);
        },
        __wbindgen_object_drop_ref: function(arg0) {
            takeObject(arg0);
        },
    };
    return {
        __proto__: null,
        "./bookworm_mobile_bg.js": import0,
    };
}

function __wasm_bindgen_func_elem_5373(arg0, arg1) {
    wasm.__wasm_bindgen_func_elem_5373(arg0, arg1);
}

function __wasm_bindgen_func_elem_5369(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_5369(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_5369_19(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_5369_19(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_6288(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_6288(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_6288_22(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_6288_22(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_5369_23(arg0, arg1, arg2) {
    wasm.__wasm_bindgen_func_elem_5369_23(arg0, arg1, addHeapObject(arg2));
}

function __wasm_bindgen_func_elem_24000(arg0, arg1, arg2) {
    try {
        wasm.__wasm_bindgen_func_elem_24000(arg0, arg1, addBorrowedObject(arg2));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

function __wasm_bindgen_func_elem_26683(arg0, arg1, arg2, arg3) {
    wasm.__wasm_bindgen_func_elem_26683(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

function __wasm_bindgen_func_elem_26662(arg0, arg1, arg2) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.__wasm_bindgen_func_elem_26662(retptr, arg0, arg1, addHeapObject(arg2));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

function __wasm_bindgen_func_elem_6284(arg0, arg1, arg2) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.__wasm_bindgen_func_elem_6284(retptr, arg0, arg1, addHeapObject(arg2));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

function __wasm_bindgen_func_elem_6284_24(arg0, arg1, arg2) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.__wasm_bindgen_func_elem_6284_24(retptr, arg0, arg1, addHeapObject(arg2));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

function __wasm_bindgen_func_elem_6284_25(arg0, arg1, arg2) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.__wasm_bindgen_func_elem_6284_25(retptr, arg0, arg1, addHeapObject(arg2));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}


const __wbindgen_enum_GpuAddressMode = ["clamp-to-edge", "repeat", "mirror-repeat"];


const __wbindgen_enum_GpuAutoLayoutMode = ["auto"];


const __wbindgen_enum_GpuBlendFactor = ["zero", "one", "src", "one-minus-src", "src-alpha", "one-minus-src-alpha", "dst", "one-minus-dst", "dst-alpha", "one-minus-dst-alpha", "src-alpha-saturated", "constant", "one-minus-constant", "src1", "one-minus-src1", "src1-alpha", "one-minus-src1-alpha"];


const __wbindgen_enum_GpuBlendOperation = ["add", "subtract", "reverse-subtract", "min", "max"];


const __wbindgen_enum_GpuBufferBindingType = ["uniform", "storage", "read-only-storage"];


const __wbindgen_enum_GpuCanvasAlphaMode = ["opaque", "premultiplied"];


const __wbindgen_enum_GpuCanvasToneMappingMode = ["standard", "extended"];


const __wbindgen_enum_GpuCompareFunction = ["never", "less", "equal", "less-equal", "greater", "not-equal", "greater-equal", "always"];


const __wbindgen_enum_GpuCullMode = ["none", "front", "back"];


const __wbindgen_enum_GpuDeviceLostReason = ["unknown", "destroyed"];


const __wbindgen_enum_GpuFilterMode = ["nearest", "linear"];


const __wbindgen_enum_GpuFrontFace = ["ccw", "cw"];


const __wbindgen_enum_GpuIndexFormat = ["uint16", "uint32"];


const __wbindgen_enum_GpuLoadOp = ["load", "clear"];


const __wbindgen_enum_GpuMipmapFilterMode = ["nearest", "linear"];


const __wbindgen_enum_GpuPowerPreference = ["low-power", "high-performance"];


const __wbindgen_enum_GpuPrimitiveTopology = ["point-list", "line-list", "line-strip", "triangle-list", "triangle-strip"];


const __wbindgen_enum_GpuSamplerBindingType = ["filtering", "non-filtering", "comparison"];


const __wbindgen_enum_GpuStencilOperation = ["keep", "zero", "replace", "invert", "increment-clamp", "decrement-clamp", "increment-wrap", "decrement-wrap"];


const __wbindgen_enum_GpuStorageTextureAccess = ["write-only", "read-only", "read-write"];


const __wbindgen_enum_GpuStoreOp = ["store", "discard"];


const __wbindgen_enum_GpuTextureAspect = ["all", "stencil-only", "depth-only"];


const __wbindgen_enum_GpuTextureDimension = ["1d", "2d", "3d"];


const __wbindgen_enum_GpuTextureFormat = ["r8unorm", "r8snorm", "r8uint", "r8sint", "r16unorm", "r16snorm", "r16uint", "r16sint", "r16float", "rg8unorm", "rg8snorm", "rg8uint", "rg8sint", "r32uint", "r32sint", "r32float", "rg16unorm", "rg16snorm", "rg16uint", "rg16sint", "rg16float", "rgba8unorm", "rgba8unorm-srgb", "rgba8snorm", "rgba8uint", "rgba8sint", "bgra8unorm", "bgra8unorm-srgb", "rgb9e5ufloat", "rgb10a2uint", "rgb10a2unorm", "rg11b10ufloat", "rg32uint", "rg32sint", "rg32float", "rgba16unorm", "rgba16snorm", "rgba16uint", "rgba16sint", "rgba16float", "rgba32uint", "rgba32sint", "rgba32float", "stencil8", "depth16unorm", "depth24plus", "depth24plus-stencil8", "depth32float", "depth32float-stencil8", "bc1-rgba-unorm", "bc1-rgba-unorm-srgb", "bc2-rgba-unorm", "bc2-rgba-unorm-srgb", "bc3-rgba-unorm", "bc3-rgba-unorm-srgb", "bc4-r-unorm", "bc4-r-snorm", "bc5-rg-unorm", "bc5-rg-snorm", "bc6h-rgb-ufloat", "bc6h-rgb-float", "bc7-rgba-unorm", "bc7-rgba-unorm-srgb", "etc2-rgb8unorm", "etc2-rgb8unorm-srgb", "etc2-rgb8a1unorm", "etc2-rgb8a1unorm-srgb", "etc2-rgba8unorm", "etc2-rgba8unorm-srgb", "eac-r11unorm", "eac-r11snorm", "eac-rg11unorm", "eac-rg11snorm", "astc-4x4-unorm", "astc-4x4-unorm-srgb", "astc-5x4-unorm", "astc-5x4-unorm-srgb", "astc-5x5-unorm", "astc-5x5-unorm-srgb", "astc-6x5-unorm", "astc-6x5-unorm-srgb", "astc-6x6-unorm", "astc-6x6-unorm-srgb", "astc-8x5-unorm", "astc-8x5-unorm-srgb", "astc-8x6-unorm", "astc-8x6-unorm-srgb", "astc-8x8-unorm", "astc-8x8-unorm-srgb", "astc-10x5-unorm", "astc-10x5-unorm-srgb", "astc-10x6-unorm", "astc-10x6-unorm-srgb", "astc-10x8-unorm", "astc-10x8-unorm-srgb", "astc-10x10-unorm", "astc-10x10-unorm-srgb", "astc-12x10-unorm", "astc-12x10-unorm-srgb", "astc-12x12-unorm", "astc-12x12-unorm-srgb"];


const __wbindgen_enum_GpuTextureSampleType = ["float", "unfilterable-float", "depth", "sint", "uint"];


const __wbindgen_enum_GpuTextureViewDimension = ["1d", "2d", "2d-array", "cube", "cube-array", "3d"];


const __wbindgen_enum_GpuVertexFormat = ["uint8", "uint8x2", "uint8x4", "sint8", "sint8x2", "sint8x4", "unorm8", "unorm8x2", "unorm8x4", "snorm8", "snorm8x2", "snorm8x4", "uint16", "uint16x2", "uint16x4", "sint16", "sint16x2", "sint16x4", "unorm16", "unorm16x2", "unorm16x4", "snorm16", "snorm16x2", "snorm16x4", "float16", "float16x2", "float16x4", "float32", "float32x2", "float32x3", "float32x4", "uint32", "uint32x2", "uint32x3", "uint32x4", "sint32", "sint32x2", "sint32x3", "sint32x4", "unorm10-10-10-2", "unorm8x4-bgra"];


const __wbindgen_enum_GpuVertexStepMode = ["vertex", "instance"];


const __wbindgen_enum_ReadableStreamType = ["bytes"];


const __wbindgen_enum_RequestCredentials = ["omit", "same-origin", "include"];


const __wbindgen_enum_RequestRedirect = ["follow", "error", "manual"];


const __wbindgen_enum_ResizeObserverBoxOptions = ["border-box", "content-box", "device-pixel-content-box"];


const __wbindgen_enum_WorkerType = ["classic", "module"];
const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr, 1));
const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr, 1));
const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr, 1));

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function addBorrowedObject(obj) {
    if (stack_pointer == 1) throw new Error('out of js stack');
    heap[--stack_pointer] = obj;
    return stack_pointer;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_export5(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function dropObject(idx) {
    if (idx < 1028) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI16FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
}

function getArrayI32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayI8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getInt8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

function getArrayJsValueViewFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(getObject(mem.getUint32(i, true)));
    }
    return result;
}

function getArrayU16FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint16ArrayMemory0().subarray(ptr / 2, ptr / 2 + len);
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

let cachedInt16ArrayMemory0 = null;
function getInt16ArrayMemory0() {
    if (cachedInt16ArrayMemory0 === null || cachedInt16ArrayMemory0.byteLength === 0) {
        cachedInt16ArrayMemory0 = new Int16Array(wasm.memory.buffer);
    }
    return cachedInt16ArrayMemory0;
}

let cachedInt32ArrayMemory0 = null;
function getInt32ArrayMemory0() {
    if (cachedInt32ArrayMemory0 === null || cachedInt32ArrayMemory0.byteLength === 0) {
        cachedInt32ArrayMemory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32ArrayMemory0;
}

let cachedInt8ArrayMemory0 = null;
function getInt8ArrayMemory0() {
    if (cachedInt8ArrayMemory0 === null || cachedInt8ArrayMemory0.byteLength === 0) {
        cachedInt8ArrayMemory0 = new Int8Array(wasm.memory.buffer);
    }
    return cachedInt8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint16ArrayMemory0 = null;
function getUint16ArrayMemory0() {
    if (cachedUint16ArrayMemory0 === null || cachedUint16ArrayMemory0.byteLength === 0) {
        cachedUint16ArrayMemory0 = new Uint16Array(wasm.memory.buffer);
    }
    return cachedUint16ArrayMemory0;
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getObject(idx) { return heap[idx]; }

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_export3(addHeapObject(e));
    }
}

let heap = new Array(1024).fill(undefined);
heap.push(undefined, null, true, false);

let heap_next = heap.length;

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_export5(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let stack_pointer = 1024;

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedInt16ArrayMemory0 = null;
    cachedInt32ArrayMemory0 = null;
    cachedInt8ArrayMemory0 = null;
    cachedUint16ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('bookworm_mobile_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
