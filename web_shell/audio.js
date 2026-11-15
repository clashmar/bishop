(() => {
  let active = null;
  let errorReported = false;
  const canvas = document.getElementById("bishop-canvas");
  const report = message => {
    if (!errorReported) {
      errorReported = true;
      window.bishopAppendStatus(`Bishop Web Audio blocked: ${message}`);
    }
  };
  const stop = session => {
    session.closed = true;
    clearInterval(session.timer);
    for (const source of session.sources) {
      try { source.stop(); } catch (_) { /* already ended */ }
      source.disconnect();
    }
    session.sources.clear();
    session.context.close().catch(() => {});
    if (active === session) active = null;
  };

  window.bishopAudioStart = (pointer, sampleRate, frameCount) => {
    const Context = window.AudioContext || window.webkitAudioContext;
    if (!Context || !window.Module?.ccall || !window.Module.HEAPF32 || !canvas) {
      report("AudioContext or WASM PCM bridge unavailable");
      return -1;
    }
    if (active) {
      if (active.context.sampleRate !== sampleRate || active.frameCount !== frameCount) {
        report("AudioContext configuration changed while playing");
        return -1;
      }
      active.pointers.push(pointer);
      return 1;
    }
    let context;
    try {
      context = new Context({ sampleRate });
    } catch (error) {
      report(`cannot create AudioContext: ${error}`);
      return -1;
    }
    if (context.sampleRate !== sampleRate) {
      report(`requires ${sampleRate} Hz, browser provided ${context.sampleRate} Hz`);
      context.close().catch(() => {});
      return -1;
    }

    const session = { context, pointers: [pointer], frameCount, closed: false, timer: null,
                      nextTime: 0, sources: new Set() };
    active = session;
    const tick = () => {
      if (session.closed || context.state !== "running") return;
      if (document.hidden || document.activeElement !== canvas) {
        session.nextTime = context.currentTime;
        return;
      }
      session.nextTime = Math.max(session.nextTime, context.currentTime);
      let queued = 0;
      while (session.nextTime < context.currentTime + 0.12 && queued < 4) {
        const address = window.Module.ccall("bishop_audio_fill", "number",
                                           ["number", "number"], [session.pointers.at(-1), frameCount]);
        if (!address) {
          report("Rust PCM callback did not return a buffer");
          stop(session);
          return;
        }
        const samples = window.Module.HEAPF32.subarray(address >>> 2,
                                                        (address >>> 2) + frameCount * 2);
        const buffer = context.createBuffer(2, frameCount, sampleRate);
        const left = buffer.getChannelData(0);
        const right = buffer.getChannelData(1);
        for (let i = 0; i < frameCount; i++) {
          left[i] = samples[i * 2];
          right[i] = samples[i * 2 + 1];
        }
        const source = context.createBufferSource();
        source.buffer = buffer;
        source.connect(context.destination);
        session.sources.add(source);
        source.onended = () => {
          session.sources.delete(source);
          source.disconnect();
        };
        source.start(session.nextTime);
        session.nextTime += frameCount / sampleRate;
        queued++;
      }
    };
    context.resume().then(() => {
      if (session.closed) return;
      window.bishopAppendStatus("Bishop Web Audio started");
      session.timer = setInterval(tick, 20);
      tick();
    }).catch(error => {
      if (session.closed) return;
      report(`cannot resume AudioContext: ${error}`);
      stop(session);
    });
    return 1;
  };

  window.bishopAudioResume = () => {
    if (!active || active.closed) return 0;
    const session = active;
    session.context.resume().catch(error => {
      if (session.closed) return;
      report(`cannot resume AudioContext: ${error}`);
      stop(session);
    });
    return 1;
  };

  window.bishopAudioStop = pointer => {
    if (active) {
      const index = active.pointers.lastIndexOf(pointer);
      if (index !== -1) {
        active.pointers.splice(index, 1);
        if (active.pointers.length === 0) stop(active);
      }
    }
    return 1;
  };
})();
