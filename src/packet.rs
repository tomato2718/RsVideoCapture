pub use rsmpeg::avcodec::AVPacket as Packet;
use rsmpeg::ffi::av_packet_clone;
use std::ptr::NonNull;

pub fn clone_packet(packet: &Packet) -> Packet {
    unsafe { Packet::from_raw(NonNull::new(av_packet_clone(packet.as_ptr())).unwrap()) }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_clone_packet() {
        let mut packet = Packet::new();
        packet.set_pts(100);
        packet.set_dts(90);
        packet.set_stream_index(1);
        packet.set_flags(1);
        packet.set_duration(10);

        let cloned = clone_packet(&packet);

        assert_eq!(cloned.pts, packet.pts);
        assert_eq!(cloned.dts, packet.dts);
        assert_eq!(cloned.stream_index, packet.stream_index);
        assert_eq!(cloned.flags, packet.flags);
        assert_eq!(cloned.duration, packet.duration);
        assert_ne!(cloned.as_ptr(), packet.as_ptr());
    }
}
