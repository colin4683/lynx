import { hash, verify } from '@node-rs/argon2';
import { encodeBase32UpperCaseNoPadding } from '@oslojs/encoding';
import { db } from '$lib/server/db';
import { users } from '$lib/server/db/schema';




export async function hashPassword(password: string): Promise<string> {
	return await hash(password, {
		memoryCost: 19456,
		timeCost: 2,
		outputLen: 32,
		parallelism: 1
	});
}

export async function verifyPassword(hash: string, password: string): Promise<boolean> {
	return await verify(hash, password);
}

export function generateRandomCode(): string {
	const recoveryCodeBytes = new Uint8Array(10);
	crypto.getRandomValues(recoveryCodeBytes);
	return encodeBase32UpperCaseNoPadding(recoveryCodeBytes);
}

export function generateEmailCode(): string {
	const recoveryCodeBytes = new Uint8Array(6);
	crypto.getRandomValues(recoveryCodeBytes);
	return encodeBase32UpperCaseNoPadding(recoveryCodeBytes);
}

export async function register(
	email: string,
	password: string
): Promise<{ success: boolean; userId?: number | null, error?: string | null }> {
	let ret = { success: false} as { success: boolean; userId?: number | null, error?: string | null };

	const existingUser = await db.query.users.findFirst({
		where: (users, { eq }) => eq(users.email, email)
	});

	if (existingUser) {
		ret.error = 'User with that email already exists.';
		return ret;
	}

	const passwordHash = await hashPassword(password);
	const code = generateEmailCode();

	let newUser = await db
		.insert(users)
		.values({
			email: email,
			password: passwordHash,
			recoveryCode: code
		})
		.returning({insertedId: users.id});

	if (!newUser.at(0)) {
		ret.error = 'Failed to create new user';
		return ret;
	}
	ret.success = true;
	ret.userId = newUser.at(0)!.insertedId;

	// todo: send email for code
	console.log(`EMAIL CODE: ${code}`);

	return ret;
}

export async function login(
	email: string,
	password: string
): Promise<{ success: boolean; userId?: number | null, error?: string | null }> {
	let ret = { success: false} as { success: boolean; userId?: number | null, error?: string | null };

	const existingUser = await db.query.users.findFirst({
		where: (users, { eq }) => eq(users.email, email)
	});

	if (!existingUser) {
		ret.error = 'Invalid credentials.';
		return ret;
	}
	ret.userId = existingUser.id;
	let verified =  await verifyPassword(existingUser!.password, password);
	ret.success = verified;
	ret.error = verified ? null : "Invalid credentials"
	return ret;
}
