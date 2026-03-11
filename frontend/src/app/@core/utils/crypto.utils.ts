import * as CryptoJS from 'crypto-js';

// 生产环境建议通过配置文件或环境变量动态获取
const KEY = CryptoJS.enc.Utf8.parse('12345678901234567890123456789012'); // 必须是32个字符
const IV = CryptoJS.enc.Utf8.parse('1234567890123456'); // 必须是16个字符

export const encryptPassword = (password: string): string => {
    if (!password) return '';
    const encrypted = CryptoJS.AES.encrypt(password, KEY, {
        iv: IV,
        mode: CryptoJS.mode.CBC,
        padding: CryptoJS.pad.Pkcs7,
    });
    return encrypted.toString(); // 返回 Base64
};
