var srcIndex = new Map(JSON.parse('[\
["async_dispatcher",["",[],["lib.rs"]]],\
["bytes",["",[["buf",[],["buf_impl.rs","buf_mut.rs","chain.rs","iter.rs","limit.rs","mod.rs","reader.rs","take.rs","uninit_slice.rs","vec_deque.rs","writer.rs"]],["fmt",[],["debug.rs","hex.rs","mod.rs"]]],["bytes.rs","bytes_mut.rs","lib.rs","loom.rs"]]],\
["cfg_if",["",[],["lib.rs"]]],\
["classicube_helpers",["",[["entities",[],["entity.rs","mod.rs"]],["events",[],["block.rs","chat.rs","entity.rs","gfx.rs","input.rs","mod.rs","net.rs","pointer.rs","tab_list.rs","texture.rs","user.rs","window.rs","world.rs"]],["tab_list",[],["entry.rs","mod.rs"]],["traits",[],["cell.rs","mod.rs","with_borrow.rs","with_inner.rs"]]],["async_manager.rs","callback_handler.rs","chat.rs","color.rs","lib.rs","macros.rs","shared.rs","tick.rs"]]],\
["classicube_sys",["",[["bitmap",[],["mod.rs","owned_bitmap.rs"]],["command",[],["mod.rs","owned_chat_command.rs"]],["core",[],["mod.rs","owned_texture.rs"]],["drawer_2d",[],["mod.rs","owned_context_2d.rs"]],["graphics",[],["mod.rs","owned_gfx_texture.rs","owned_vertex_buffer.rs"]],["screen",[],["mod.rs","priority.rs"]],["vectors",[["ivec",[],["mod.rs","ops.rs"]],["matrix",[],["mod.rs","ops.rs"]],["vec",[],["mod.rs","ops.rs"]]],["mod.rs"]]],["bindings.rs","chat.rs","constants.rs","entity.rs","event.rs","input.rs","inventory.rs","lib.rs","math.rs","model.rs","packed_col.rs","particle.rs","std_types.rs","string.rs","world.rs"]]],\
["futures",["",[],["lib.rs"]]],\
["futures_channel",["",[["mpsc",[],["mod.rs","queue.rs","sink_impl.rs"]]],["lib.rs","lock.rs","oneshot.rs"]]],\
["futures_core",["",[["task",[["__internal",[],["atomic_waker.rs","mod.rs"]]],["mod.rs","poll.rs"]]],["future.rs","lib.rs","stream.rs"]]],\
["futures_executor",["",[],["enter.rs","lib.rs","local_pool.rs"]]],\
["futures_io",["",[],["lib.rs"]]],\
["futures_macro",["",[],["executor.rs","join.rs","lib.rs","select.rs","stream_select.rs"]]],\
["futures_sink",["",[],["lib.rs"]]],\
["futures_task",["",[],["arc_wake.rs","future_obj.rs","lib.rs","noop_waker.rs","spawn.rs","waker.rs","waker_ref.rs"]]],\
["futures_timer",["",[["native",[],["arc_list.rs","atomic_waker.rs","delay.rs","global.rs","heap.rs","heap_timer.rs","timer.rs"]]],["lib.rs","native.rs"]]],\
["futures_util",["",[["async_await",[],["join_mod.rs","mod.rs","pending.rs","poll.rs","random.rs","select_mod.rs","stream_select_mod.rs"]],["future",[["future",[],["catch_unwind.rs","flatten.rs","fuse.rs","map.rs","mod.rs","remote_handle.rs","shared.rs"]],["try_future",[],["into_future.rs","mod.rs","try_flatten.rs","try_flatten_err.rs"]]],["abortable.rs","either.rs","join.rs","join_all.rs","lazy.rs","maybe_done.rs","mod.rs","option.rs","pending.rs","poll_fn.rs","poll_immediate.rs","ready.rs","select.rs","select_all.rs","select_ok.rs","try_join.rs","try_join_all.rs","try_maybe_done.rs","try_select.rs"]],["io",[],["allow_std.rs","buf_reader.rs","buf_writer.rs","chain.rs","close.rs","copy.rs","copy_buf.rs","copy_buf_abortable.rs","cursor.rs","empty.rs","fill_buf.rs","flush.rs","into_sink.rs","line_writer.rs","lines.rs","mod.rs","read.rs","read_exact.rs","read_line.rs","read_to_end.rs","read_to_string.rs","read_until.rs","read_vectored.rs","repeat.rs","seek.rs","sink.rs","split.rs","take.rs","window.rs","write.rs","write_all.rs","write_vectored.rs"]],["lock",[],["bilock.rs","mod.rs","mutex.rs"]],["sink",[],["buffer.rs","close.rs","drain.rs","err_into.rs","fanout.rs","feed.rs","flush.rs","map_err.rs","mod.rs","send.rs","send_all.rs","unfold.rs","with.rs","with_flat_map.rs"]],["stream",[["futures_unordered",[],["abort.rs","iter.rs","mod.rs","ready_to_run_queue.rs","task.rs"]],["stream",[],["all.rs","any.rs","buffer_unordered.rs","buffered.rs","catch_unwind.rs","chain.rs","chunks.rs","collect.rs","concat.rs","count.rs","cycle.rs","enumerate.rs","filter.rs","filter_map.rs","flatten.rs","flatten_unordered.rs","fold.rs","for_each.rs","for_each_concurrent.rs","forward.rs","fuse.rs","into_future.rs","map.rs","mod.rs","next.rs","peek.rs","ready_chunks.rs","scan.rs","select_next_some.rs","skip.rs","skip_while.rs","split.rs","take.rs","take_until.rs","take_while.rs","then.rs","unzip.rs","zip.rs"]],["try_stream",[],["and_then.rs","into_async_read.rs","into_stream.rs","mod.rs","or_else.rs","try_all.rs","try_any.rs","try_buffer_unordered.rs","try_buffered.rs","try_chunks.rs","try_collect.rs","try_concat.rs","try_filter.rs","try_filter_map.rs","try_flatten.rs","try_flatten_unordered.rs","try_fold.rs","try_for_each.rs","try_for_each_concurrent.rs","try_next.rs","try_ready_chunks.rs","try_skip_while.rs","try_take_while.rs","try_unfold.rs"]]],["abortable.rs","empty.rs","futures_ordered.rs","iter.rs","mod.rs","once.rs","pending.rs","poll_fn.rs","poll_immediate.rs","repeat.rs","repeat_with.rs","select.rs","select_all.rs","select_with_strategy.rs","unfold.rs"]],["task",[],["mod.rs","spawn.rs"]]],["abortable.rs","fns.rs","lib.rs","never.rs","unfold_state.rs"]]],\
["lazy_static",["",[],["inline_lazy.rs","lib.rs"]]],\
["lock_api",["",[],["lib.rs","mutex.rs","remutex.rs","rwlock.rs"]]],\
["memchr",["",[["arch",[["all",[["packedpair",[],["default_rank.rs","mod.rs"]]],["memchr.rs","mod.rs","rabinkarp.rs","shiftor.rs","twoway.rs"]],["generic",[],["memchr.rs","mod.rs","packedpair.rs"]],["x86_64",[["avx2",[],["memchr.rs","mod.rs","packedpair.rs"]],["sse2",[],["memchr.rs","mod.rs","packedpair.rs"]]],["memchr.rs","mod.rs"]]],["mod.rs"]],["memmem",[],["mod.rs","searcher.rs"]]],["cow.rs","ext.rs","lib.rs","macros.rs","memchr.rs","vector.rs"]]],\
["mio",["",[["event",[],["event.rs","events.rs","mod.rs","source.rs"]],["net",[["tcp",[],["listener.rs","mod.rs","stream.rs"]]],["mod.rs","udp.rs"]],["sys",[["windows",[],["afd.rs","event.rs","handle.rs","io_status_block.rs","iocp.rs","mod.rs","named_pipe.rs","net.rs","overlapped.rs","selector.rs","tcp.rs","udp.rs","waker.rs"]]],["mod.rs"]]],["interest.rs","io_source.rs","lib.rs","macros.rs","poll.rs","token.rs","waker.rs"]]],\
["once_cell",["",[],["imp_std.rs","lib.rs","race.rs"]]],\
["parking_lot",["",[],["condvar.rs","deadlock.rs","elision.rs","fair_mutex.rs","lib.rs","mutex.rs","once.rs","raw_fair_mutex.rs","raw_mutex.rs","raw_rwlock.rs","remutex.rs","rwlock.rs","util.rs"]]],\
["parking_lot_core",["",[["thread_parker",[["windows",[],["bindings.rs","keyed_event.rs","mod.rs","waitaddress.rs"]]],["mod.rs"]]],["lib.rs","parking_lot.rs","spinwait.rs","util.rs","word_lock.rs"]]],\
["paste",["",[],["attr.rs","error.rs","lib.rs","segment.rs"]]],\
["pin_project_lite",["",[],["lib.rs"]]],\
["pin_utils",["",[],["lib.rs","projection.rs","stack_pin.rs"]]],\
["proc_macro2",["",[],["detection.rs","extra.rs","fallback.rs","lib.rs","marker.rs","parse.rs","rcvec.rs","wrapper.rs"]]],\
["quote",["",[],["ext.rs","format.rs","ident_fragment.rs","lib.rs","runtime.rs","spanned.rs","to_tokens.rs"]]],\
["scopeguard",["",[],["lib.rs"]]],\
["slab",["",[],["builder.rs","lib.rs"]]],\
["smallvec",["",[],["lib.rs"]]],\
["socket2",["",[["sys",[],["windows.rs"]]],["lib.rs","sockaddr.rs","socket.rs","sockref.rs"]]],\
["syn",["",[["gen",[],["clone.rs","debug.rs","eq.rs","hash.rs","visit_mut.rs"]]],["attr.rs","bigint.rs","buffer.rs","classify.rs","custom_keyword.rs","custom_punctuation.rs","data.rs","derive.rs","discouraged.rs","drops.rs","error.rs","export.rs","expr.rs","ext.rs","file.rs","fixup.rs","generics.rs","group.rs","ident.rs","item.rs","lib.rs","lifetime.rs","lit.rs","lookahead.rs","mac.rs","macros.rs","meta.rs","op.rs","parse.rs","parse_macro_input.rs","parse_quote.rs","pat.rs","path.rs","precedence.rs","print.rs","punctuated.rs","restriction.rs","sealed.rs","span.rs","spanned.rs","stmt.rs","thread.rs","token.rs","tt.rs","ty.rs","verbatim.rs","whitespace.rs"]]],\
["tokio",["",[["fs",[],["canonicalize.rs","copy.rs","create_dir.rs","create_dir_all.rs","dir_builder.rs","file.rs","hard_link.rs","metadata.rs","mod.rs","open_options.rs","read.rs","read_dir.rs","read_link.rs","read_to_string.rs","remove_dir.rs","remove_dir_all.rs","remove_file.rs","rename.rs","set_permissions.rs","symlink_dir.rs","symlink_file.rs","symlink_metadata.rs","try_exists.rs","write.rs"]],["future",[],["block_on.rs","maybe_done.rs","mod.rs","poll_fn.rs","try_join.rs"]],["io",[["util",[],["async_buf_read_ext.rs","async_read_ext.rs","async_seek_ext.rs","async_write_ext.rs","buf_reader.rs","buf_stream.rs","buf_writer.rs","chain.rs","copy.rs","copy_bidirectional.rs","copy_buf.rs","empty.rs","fill_buf.rs","flush.rs","lines.rs","mem.rs","mod.rs","read.rs","read_buf.rs","read_exact.rs","read_int.rs","read_line.rs","read_to_end.rs","read_to_string.rs","read_until.rs","repeat.rs","shutdown.rs","sink.rs","split.rs","take.rs","vec_with_initialized.rs","write.rs","write_all.rs","write_all_buf.rs","write_buf.rs","write_int.rs","write_vectored.rs"]]],["async_buf_read.rs","async_read.rs","async_seek.rs","async_write.rs","blocking.rs","interest.rs","join.rs","mod.rs","poll_evented.rs","read_buf.rs","ready.rs","seek.rs","split.rs","stderr.rs","stdin.rs","stdio_common.rs","stdout.rs"]],["loom",[["std",[],["atomic_u16.rs","atomic_u32.rs","atomic_u64.rs","atomic_u64_native.rs","atomic_usize.rs","barrier.rs","mod.rs","mutex.rs","parking_lot.rs","unsafe_cell.rs"]]],["mod.rs"]],["macros",[],["addr_of.rs","cfg.rs","join.rs","loom.rs","mod.rs","pin.rs","select.rs","support.rs","thread_local.rs","try_join.rs"]],["net",[["tcp",[],["listener.rs","mod.rs","socket.rs","split.rs","split_owned.rs","stream.rs"]],["windows",[],["mod.rs","named_pipe.rs"]]],["addr.rs","lookup_host.rs","mod.rs","udp.rs"]],["process",[],["kill.rs","mod.rs","windows.rs"]],["runtime",[["blocking",[],["mod.rs","pool.rs","schedule.rs","shutdown.rs","task.rs"]],["context",[],["blocking.rs","current.rs","runtime.rs","runtime_mt.rs","scoped.rs"]],["io",[],["driver.rs","metrics.rs","mod.rs","registration.rs","registration_set.rs","scheduled_io.rs"]],["metrics",[],["mock.rs","mod.rs","runtime.rs"]],["scheduler",[["current_thread",[],["mod.rs"]],["inject",[],["pop.rs","rt_multi_thread.rs","shared.rs","synced.rs"]],["multi_thread",[["handle",[],["metrics.rs"]],["worker",[],["taskdump_mock.rs"]]],["counters.rs","handle.rs","idle.rs","mod.rs","overflow.rs","park.rs","queue.rs","stats.rs","trace_mock.rs","worker.rs"]]],["block_in_place.rs","defer.rs","inject.rs","lock.rs","mod.rs"]],["task",[],["abort.rs","core.rs","error.rs","harness.rs","id.rs","join.rs","list.rs","mod.rs","raw.rs","state.rs","waker.rs"]],["time",[["wheel",[],["level.rs","mod.rs"]]],["entry.rs","handle.rs","mod.rs","source.rs"]]],["builder.rs","config.rs","context.rs","coop.rs","driver.rs","handle.rs","mod.rs","park.rs","runtime.rs","task_hooks.rs","thread_id.rs"]],["signal",[["windows",[],["sys.rs"]]],["ctrl_c.rs","mod.rs","registry.rs","reusable_box.rs","windows.rs"]],["sync",[["mpsc",[],["block.rs","bounded.rs","chan.rs","error.rs","list.rs","mod.rs","unbounded.rs"]],["rwlock",[],["owned_read_guard.rs","owned_write_guard.rs","owned_write_guard_mapped.rs","read_guard.rs","write_guard.rs","write_guard_mapped.rs"]],["task",[],["atomic_waker.rs","mod.rs"]]],["barrier.rs","batch_semaphore.rs","broadcast.rs","mod.rs","mutex.rs","notify.rs","once_cell.rs","oneshot.rs","rwlock.rs","semaphore.rs","watch.rs"]],["task",[],["blocking.rs","consume_budget.rs","join_set.rs","local.rs","mod.rs","spawn.rs","task_local.rs","unconstrained.rs","yield_now.rs"]],["time",[],["clock.rs","error.rs","instant.rs","interval.rs","mod.rs","sleep.rs","timeout.rs"]],["util",[["rand",[],["rt.rs"]]],["atomic_cell.rs","bit.rs","cacheline.rs","error.rs","idle_notified_set.rs","linked_list.rs","markers.rs","memchr.rs","metric_atomics.rs","mod.rs","once_cell.rs","rand.rs","rc_cell.rs","sharded_list.rs","sync_wrapper.rs","trace.rs","try_lock.rs","wake.rs","wake_list.rs"]]],["blocking.rs","lib.rs"]]],\
["tokio_macros",["",[],["entry.rs","lib.rs","select.rs"]]],\
["tracing",["",[],["dispatcher.rs","field.rs","instrument.rs","level_filters.rs","lib.rs","macros.rs","span.rs","stdlib.rs","subscriber.rs"]]],\
["tracing_attributes",["",[],["attr.rs","expand.rs","lib.rs"]]],\
["tracing_core",["",[],["callsite.rs","dispatcher.rs","event.rs","field.rs","lazy.rs","lib.rs","metadata.rs","parent.rs","span.rs","stdlib.rs","subscriber.rs"]]],\
["unicode_ident",["",[],["lib.rs","tables.rs"]]],\
["windows_targets",["",[],["lib.rs"]]],\
["windows_x86_64_msvc",["",[],["lib.rs"]]]\
]'));
createSrcSidebar();
