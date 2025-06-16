import { decrypt, encrypt } from '$lib/server/encryption';
import { db } from '$lib/server/db';
import { users } from '$lib/server/db/schema';
import { eq } from 'drizzle-orm';

export interface User {
	id: number;
	email: string;
	emailVerified: boolean | null;
	registered2FA: boolean;
}


export async function getUserTOTPKey(userId: number): Promise<Uint8Array | null> {
	let user = await db.query.users.findFirst({where: (users, {eq}) => eq(users.id, userId)});
	if (!user || !user.totpKey) {
		return null;
	}
	return decrypt(user.totpKey);
}

export async function updateUserTOTPKey(userId: number, key: Uint8Array) : Promise<void> {
	const encrypted = encrypt(key);
	await db.update(users).set({
		totpKey:  Buffer.from(encrypted)
	}).where(eq(users.id, userId));
}