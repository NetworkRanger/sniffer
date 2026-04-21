use std::borrow::Cow;
use std::fs::File;
use pcap_file::DataLink;
use pcap::PacketHeader as PcapPacketHeader;
use pcap_file::pcap::{Packet, PacketHeader, PcapHeader};

pub struct PcapWriter {
    rx: async_channel::Receiver<(PcapPacketHeader, Vec<u8>)>,
    writer: pcap_file::PcapWriter<File>,
}

impl PcapWriter {
    pub fn new(rx: async_channel::Receiver<(PcapPacketHeader, Vec<u8>)>) -> Self {
        let file_path = dirs::home_dir().map(|home| home.join(".sniffer/capture.pcap"))
            .unwrap().to_string_lossy().into_owned();
        let file = File::create(&file_path).unwrap();
        let header = PcapHeader {
            magic_number: 0xa1b2c3d4,
            version_major: 2,
            version_minor: 4,
            snaplen: 65535,
            ts_correction: 0,
            ts_accuracy: 0,
            datalink: DataLink::ETHERNET,
        };
        let writer = pcap_file::PcapWriter::with_header(header, file).unwrap();
        Self {
            rx,
            writer,
        }
    }
    
    pub fn start(&mut self){
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            while let Ok((header, data)) = self.rx.recv().await {
                let packet = Packet{
                    header: PacketHeader {
                        ts_sec: header.ts.tv_sec as u32,
                        ts_nsec: (header.ts.tv_usec as u32) * 1000,
                        incl_len: header.caplen,
                        orig_len: header.len,
                    },
                    data: Cow::from(data),
                };
                self.writer.write_packet(&packet).unwrap();
            }
        });
    }
}