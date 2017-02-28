use std::{thread, time};
use i2cdev::core::I2CDevice;
use i2cdev::linux::{LinuxI2CError};

use super::calibration::Calibration;
use super::register::Register;


const BME280OSAMPLE1 : u8 = 1;
const BME280OSAMPLE2 : u8 = 2;
const BME280OSAMPLE4 : u8 = 3;
const BME280OSAMPLE8 : u8 = 4;
const BME280OSAMPLE16 : u8 = 5;

const MAX_OVER_SAMPLING_AND_NORMAL_MODE : u8 = 0x3F;

pub struct Bme280<'a, T: I2CDevice<Error=LinuxI2CError> + Sized + 'a> {
    calibration: Calibration,
    device: &'a mut T,
    mode: u8
}

impl<'a, T: I2CDevice<Error=LinuxI2CError> + Sized + 'a> Bme280<'a, T> {

    pub fn new(dev: &'a mut T) -> Result<Bme280<'a, T>, LinuxI2CError> {
        let cal = try!(Bme280::get_calibration(dev));
        try!(dev.smbus_write_byte_data(Register::CONTROL as u8, MAX_OVER_SAMPLING_AND_NORMAL_MODE));
        Ok(Bme280 { calibration: cal, device: dev, mode: BME280OSAMPLE1 })
    }

    pub fn print_calibration(&mut self) {
        println!("{:?}", self.calibration);
    }

    fn get_calibration(dev: &mut T) -> Result<Calibration, LinuxI2CError> {
        println!("Reading calibration:");
        Ok(Calibration {
            t1: try!(Bme280::readWord(dev, Register::T1)),
            t2: try!(Bme280::readWord(dev, Register::T2)),
            t3: try!(Bme280::readWord(dev, Register::T2)),

            p1: try!(Bme280::readWord(dev, Register::T1)),
            p2: try!(Bme280::readWord(dev, Register::T1)),
            p3: try!(Bme280::readWord(dev, Register::T1)),
            p4: try!(Bme280::readWord(dev, Register::T1)),
            p5: try!(Bme280::readWord(dev, Register::T1)),
            p6: try!(Bme280::readWord(dev, Register::T1)),
            p7: try!(Bme280::readWord(dev, Register::T1)),
            p8: try!(Bme280::readWord(dev, Register::T1)),
            p9: try!(Bme280::readWord(dev, Register::T1)),

            h1: try!(Bme280::readWord(dev, Register::T1)),
            h2: try!(Bme280::readWord(dev, Register::T1)),
            h3: try!(Bme280::readWord(dev, Register::T1)),
            h7: try!(Bme280::readWord(dev, Register::T1))
        })
    }

    fn readWord(dev: &mut T, register: Register) -> Result<i16, LinuxI2CError> {
        let dig = try!(dev.smbus_read_word_data(register as u8));
        println!("{}", dig);
        Ok(dig as i16)
    }

    fn read_raw_temp(&mut self) -> Result<f64, LinuxI2CError> { 
        try!(self.device.smbus_write_byte_data(Register::CONTROL_HUM as u8, self.mode));
        let meas = self.mode << 5 | self.mode << 2 | 1;
        try!(self.device.smbus_write_byte_data(Register::CONTROL as u8, meas));
        let mut sleep_time = 0.00125 + 0.0023 * (1 << self.mode) as f32;
        sleep_time = sleep_time + 0.0023 * (1 << self.mode) as f32 + 0.000575;
        sleep_time = sleep_time + 0.0023 * (1 << self.mode) as f32 + 0.000575;
        let dur = time::Duration::from_millis((sleep_time * 1000.0) as u64);
        thread::sleep(dur);
        
        println!("Reading raw temp data:");
        let msb = try!(self.readByteData(Register::TEMP_DATA));
        let lsb = try!(self.readByteData(Register::TEMP_DATA_1));
        let xlsb = try!(self.readByteData(Register::TEMP_DATA_2));
        let raw = ((msb << 16) | (lsb << 8) | xlsb) >> 4;
        Ok(raw as f64)
    }

    fn readByteData(&mut self, register: Register) -> Result<u64, LinuxI2CError> {
        let val = try!(self.device.smbus_read_byte_data(register as u8));
        println!("{}", val);
        Ok(val as u64)
    }

    fn calc_t_fine(&mut self) -> Result<f64, LinuxI2CError> {
        let ut = try!(self.read_raw_temp());
        let t1 = self.calibration.t1 as f64;
        let t2 = self.calibration.t2 as f64;
        let t3 = self.calibration.t3 as f64;
        let var1 = (ut / 16384.0 - t1 / 1024.0) * t2;
        let var2 = ((ut / 131072.0 - t1 / 8192.0) * (ut / 131072.0 - t1 / 8192.0)) * t3;
        let t_fine = var1 + var2;
        Ok(t_fine)
    }

    pub fn read_temperature(&mut self) -> Result<f64, LinuxI2CError> {
        // Technically I'm skipping the step of casting to an integer, which would
        // result in rounding down of the var1 and var2 that were used in the original
        // calculation of t_fine:
        let celcius = try!(self.calc_t_fine()) / 5120.0;
        let fahrenheit = celcius * 1.8 + 32.0;
        Ok(fahrenheit)
    }

    fn read_raw_pressure(&mut self) -> Result<u32, LinuxI2CError> {
        let msb = try!(self.device.smbus_read_byte_data(Register::PRESSURE_DATA as u8)) as u32;
        let lsb = try!(self.device.smbus_read_byte_data(Register::PRESSURE_DATA as u8)) as u32;
        let xlsb = try!(self.device.smbus_read_byte_data(Register::PRESSURE_DATA as u8)) as u32;
        let raw = ((msb << 16) | (lsb << 8) | xlsb) >> 4;
        // println!("Raw is: {}", raw);
        Ok(raw)
    }

    pub fn read_pressure(&mut self) -> Result<f64, LinuxI2CError> {
        let p1 = self.calibration.p1 as f64;
        let p2 = self.calibration.p2 as f64;
        let p3 = self.calibration.p3 as f64;
        let p4 = self.calibration.p4 as f64;
        let p5 = self.calibration.p5 as f64;
        let p6 = self.calibration.p6 as f64;
        let p7 = self.calibration.p7 as f64;
        let p8 = self.calibration.p8 as f64;
        let p9 = self.calibration.p9 as f64;

        let adc = try!(self.read_raw_pressure()) as f64;
        let t_fine = try!(self.calc_t_fine());
        // println!("t_fine: {}", t_fine);
        
        let var1 = t_fine / 2.0 - 64000.0;
        // println!("var1: {}", var1);

        let var2 = var1 * var1 * p6 / 32768.0;
        // println!("var2: {}", var2);

        let var2_2 = var2 + var1 * p5 * 2.0;
        // println!("var2_2: {}", var2_2);
    
        let var2_3 = var2_2 / 4.0 + p4 * 65536.0;
        // println!("var2_3: {}", var2_3);

        let var1_2 = (p3 * var1 * var1 / 524288.0 + p2 * var1) / 524288.0;
        // println!("var1_2: {}", var1_2);

        let var1_3 = (1.0 + var1_2 / 32768.0) * p1;
        // println!("var1_3: {}", var1_3);

        if var1_3 == 0.0 {
            return Ok(0.0);
        }

        let p = 1048576.0 - adc;
        // println!("p: {}", p);

        let p_2 = ((p - var2_3 / 4096.0) * 6250.0) / var1_3;
        // println!("p_2: {}", p_2);

        let var1_4 = p9 * p_2 * p_2 / 2147483648.0;
        // println!("var1_4: {}", var1_4);

        let var2_4 = p_2 * p8 / 32768.0;
        // println!("var2_4: {}", var2_4);

        let pascals = p_2 + (var1_4 + var2_4 + p7) / 16.0;
        // println!("pascals: {}", pascals);

        // println!("Calibration: {}", self.Calibration);

        let in_hg = pascals *  0.000295299830714;

        Ok(in_hg)
    }    
}

#[cfg(test)]
mod tests {

    use i2cdev::core::I2CDevice;
    use i2cdev::linux::{LinuxI2CError};
    use std::io::{Error, ErrorKind};
    use nix;
    use bme280::{Bme280};
    use super::super::register::Register;

    struct FakeDevice {

    }

    impl I2CDevice for FakeDevice {
        type Error = LinuxI2CError;

        /// Read data from the device to fill the provided slice
        fn read(&mut self, data: &mut [u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        /// Write the provided buffer to the device
        fn write(&mut self, data: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        /// This sends a single bit to the device, at the place of the Rd/Wr bit
        fn smbus_write_quick(&mut self, bit: bool) -> Result<(), Self::Error> {
            Ok(())
        }

        /// Read a block of up to 32 bytes from a device
        ///
        /// The actual number of bytes available to read is returned in the count
        /// byte.  This code returns a correctly sized vector containing the
        /// count bytes read from the device.
        fn smbus_read_block_data(&mut self, register: u8) -> Result<Vec<u8>, Self::Error> {
            Ok(vec![1,2,3])
        }

        /// Read a block of up to 32 bytes from a device
        ///
        /// Uses read_i2c_block_data instead read_block_data.
        fn smbus_read_i2c_block_data(&mut self, register: u8, len: u8) -> Result<Vec<u8>, Self::Error> {
            Ok(vec![1,2,3])
        }

        /// Write a block of up to 32 bytes to a device
        ///
        /// The opposite of the Block Read command, this writes up to 32 bytes to
        /// a device, to a designated register that is specified through the
        /// Comm byte. The amount of data is specified in the Count byte.
        fn smbus_write_block_data(&mut self, register: u8, values: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        /// Select a register, send 1 to 31 bytes of data to it, and reads
        /// 1 to 31 bytes of data from it.
        fn smbus_process_block(&mut self, register: u8, values: &[u8]) -> Result<(), Self::Error> {
            Ok(())
        }
    
        /// Read 2 bytes form a given register on a device
        fn smbus_read_word_data(&mut self, register: u8) -> Result<u16, LinuxI2CError> {
            match register {
                register if register == Register::T1 as u8 => Ok(28960),
                register if register == Register::T2 as u8 => Ok(26619),
                register if register == Register::T3 as u8 => Ok(26619),
                register if register == Register::P1 as u8 => Ok(28960),
                register if register == Register::P2 as u8 => Ok(28960),
                register if register == Register::P3 as u8 => Ok(28960),
                register if register == Register::P4 as u8 => Ok(28960),
                register if register == Register::P5 as u8 => Ok(28960),
                register if register == Register::P6 as u8 => Ok(28960),
                register if register == Register::P7 as u8 => Ok(28960),
                register if register == Register::P8 as u8 => Ok(28960),
                register if register == Register::P9 as u8 => Ok(28960),
                register if register == Register::H1 as u8 => Ok(28960),
                register if register == Register::H2 as u8 => Ok(28960),
                register if register == Register::H3 as u8 => Ok(28960),
                register if register == Register::H7 as u8 => Ok(28960),
                _ => Err(LinuxI2CError::Nix(nix::Error::InvalidPath))
            }
        }

        fn smbus_read_byte_data(&mut self, register: u8) -> Result<u8, Self::Error> {
             match register {
                register if register == Register::TEMP_DATA as u8 => Ok(129),
                register if register == Register::TEMP_DATA_1 as u8 => Ok(142),
                register if register == Register::TEMP_DATA_2 as u8 => Ok(0),
                _ => Err(LinuxI2CError::Nix(nix::Error::InvalidPath))
            }
        }
    }

    #[test]
    fn fn_set_of_known_calibration_values_should_yield_known_temperature() {        
        let mut device = FakeDevice {};
        let mut result = Bme280::new(&mut device).unwrap();

        let t = result.read_temperature().unwrap();
        assert!((t - 72.91).abs() < 0.01);
    }
}

// test temperature_reading_should_be_reasonable ... Reading calibration:
// 28960
// 26619
// 26619
// 28960
// 28960
// 28960
// 28960
// 28960
// 28960
// 28960
// 28960
// 28960
// 28960
// 28960
// 28960
// 28960
// Reading raw temp data:
// 129
// 142
// 0
// The temperature is: 72.91



// test pressure_reading_should_be_reasonable ... The pressure is: 29.61 in hg.
// test print_the_calibration ... (t1:28960, t2:26619, t3:50, p1:-10713, p2:-10713, p3:3024, p4:5831, p5:96, p6:-7, p7:9900, p8:-10230, p9:4285, h1:75, h2:355, h3:0, h7:30)
// test temperature_reading_should_be_reasonable ... The temperature is: 67.03

