using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Threading;

// Windows x64 SDK ABI. Only network metadata is consumed; no packet payload is read.
internal sealed class NativeEtw : IDisposable
{
    [UnmanagedFunctionPointer(CallingConvention.Winapi)]
    private delegate void EventCallback(IntPtr record);
    [StructLayout(LayoutKind.Explicit, Size = 448)]
    private struct LogFile {
        [FieldOffset(8)] public IntPtr LoggerName;
        [FieldOffset(28)] public uint Mode;
        [FieldOffset(424)] public IntPtr Callback;
    }
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
    private static extern uint StartTraceW(out ulong session, string name, IntPtr properties);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode)]
    private static extern uint ControlTraceW(ulong session, string name, IntPtr properties, uint control);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern ulong OpenTraceW(ref LogFile log);
    [DllImport("advapi32.dll")]
    private static extern uint ProcessTrace(ulong[] handles, uint count, IntPtr start, IntPtr end);
    [DllImport("advapi32.dll")]
    private static extern uint CloseTrace(ulong handle);
    private readonly string name;
    private readonly Guid identity;
    private readonly EventCallback callback;
    private readonly object gate = new object();
    private readonly IntPtr properties;
    private readonly IntPtr loggerName;
    private ulong session;
    private ulong consumer = ulong.MaxValue;
    private Thread thread;
    private bool disposed;
    private long malformed;
    private uint processError;
    internal Action<uint, uint, bool, long> OnBytes;
    private static readonly Guid Tcp = new Guid("9a280ac0-c8e0-11d1-84e2-00c04fb998a2");
    private static readonly Guid Udp = new Guid("bf3a50c5-a9c9-4988-a005-2df0b7c80f80");

    internal NativeEtw(Guid identity) {
        if (IntPtr.Size != 8) throw new NotSupportedException("Windows x64 is required");
        this.identity = identity;
        name = "Pinmeter-network-" + identity.ToString("N");
        properties = Marshal.AllocHGlobal(120 + 1024);
        ResetProperties();
        Marshal.WriteInt32(properties, 40, 1); // QPC; OpenTrace converts timestamps to FILETIME.
        Marshal.WriteInt32(properties, 48, 64); // 64 KiB buffers
        Marshal.WriteInt32(properties, 52, 16);
        Marshal.WriteInt32(properties, 56, 64); // Request 4 MiB; Windows may raise it for large CPU counts.
        Marshal.WriteInt32(properties, 64, 0x02000100); // SYSTEM_LOGGER | REAL_TIME
        Marshal.WriteInt32(properties, 68, 1);
        Marshal.WriteInt32(properties, 72, 0x10000); // NETWORK_TCPIP only
        callback = Receive;
        loggerName = Marshal.StringToHGlobalUni(name);
    }
    private void ResetProperties() {
        Marshal.Copy(new byte[120 + 1024], 0, properties, 120 + 1024);
        Marshal.WriteInt32(properties, 0, 120 + 1024);
        Marshal.StructureToPtr(identity, IntPtr.Add(properties, 24), false);
        Marshal.WriteInt32(properties, 44, 0x20000); // WNODE_FLAG_TRACED_GUID
        Marshal.WriteInt32(properties, 116, 120);
    }
    internal void Start() {
        uint error = StartTraceW(out session, name, properties);
        if (error != 0) { session = 0; throw new Win32Exception((int)error); }
        var log = new LogFile { LoggerName = loggerName, Mode = 0x10000100,
            Callback = Marshal.GetFunctionPointerForDelegate(callback) }; // EVENT_RECORD | REAL_TIME
        consumer = OpenTraceW(ref log);
        if (consumer == ulong.MaxValue) throw new Win32Exception(Marshal.GetLastWin32Error());
        ulong consumerHandle = consumer;
        thread = new Thread(() => { processError = ProcessTrace(new[] { consumerHandle }, 1, IntPtr.Zero, IntPtr.Zero); });
        thread.IsBackground = true;
        thread.Start();
    }
    private void Receive(IntPtr record) {
        try {
            var provider = (Guid)Marshal.PtrToStructure(IntPtr.Add(record, 24), typeof(Guid));
            if (provider != Tcp && provider != Udp) return;
            byte opcode = Marshal.ReadByte(record, 45);
            if (opcode != 10 && opcode != 11 && opcode != 26 && opcode != 27) return;
            byte version = Marshal.ReadByte(record, 42);
            int length = (ushort)Marshal.ReadInt16(record, 86);
            // Both providers' version 2 layouts begin with payload PID and byte count.
            if (version != 2 || length < 8) { Interlocked.Increment(ref malformed); return; }
            IntPtr data = Marshal.ReadIntPtr(record, 96);
            uint pid = unchecked((uint)Marshal.ReadInt32(data));
            uint size = unchecked((uint)Marshal.ReadInt32(data, 4));
            var handler = OnBytes;
            if (handler != null) handler(pid, size, opcode == 10 || opcode == 26, Marshal.ReadInt64(record, 16));
        } catch { Interlocked.Increment(ref malformed); }
    }
    internal ulong Lost() {
        lock (gate) {
            if (disposed || session == 0) throw new InvalidOperationException("ETW session closed");
            ResetProperties();
            uint error = ControlTraceW(session, name, properties, 0);
            if (error != 0) throw new Win32Exception((int)error);
            if (thread != null && !thread.IsAlive) throw new Win32Exception((int)processError, "ETW consumer stopped");
            return (uint)Marshal.ReadInt32(properties, 88) + (ulong)(uint)Marshal.ReadInt32(properties, 100)
                + (ulong)Interlocked.Read(ref malformed);
        }
    }
    public void Dispose() {
        lock (gate) {
            if (disposed) return;
            if (session != 0) {
                ResetProperties();
                uint error = ControlTraceW(session, name, properties, 1);
                // MORE_DATA still means STOP succeeded; INSTANCE_NOT_FOUND is already stopped.
                if (error != 0 && error != 234 && error != 4201) throw new Win32Exception((int)error, "ETW session stop failed");
                session = 0;
            }
            if (consumer != ulong.MaxValue) {
                uint error = CloseTrace(consumer);
                // CTX_CLOSE_PENDING: ProcessTrace drains queued events before returning.
                if (error != 0 && error != 7007) throw new Win32Exception((int)error, "ETW consumer close failed");
                consumer = ulong.MaxValue;
            }
            if (thread != null && !thread.Join(3000)) throw new TimeoutException("ETW consumer did not stop"); // Exit the helper; never reuse a live old callback.
            Marshal.FreeHGlobal(properties);
            Marshal.FreeHGlobal(loggerName);
            GC.KeepAlive(callback);
            disposed = true;
        }
    }
}
